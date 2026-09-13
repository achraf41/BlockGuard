use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use blockguard_chain::{
    BlockMetadata, Blockchain, ChainError, ChainWork, GenesisAllocation, GenesisConfig,
    GenesisConfigError,
};
use blockguard_core::{
    Address, Amount, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion,
    ChainId, Hash256, MerkleRoot, Nonce, PowNonce, PowTarget, PublicKeyBytes, SignatureBytes,
    SignedTransaction, StateRoot, TransactionVersion, UnsignedTransaction,
};
use blockguard_crypto::block_hash;
use blockguard_state::{Account, State};

const MAGIC: &[u8; 8] = b"BGSTORE1";
const STORAGE_VERSION: u16 = 1;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    InvalidData(&'static str),
    Chain(ChainError),
    Genesis(GenesisConfigError),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "storage I/O failed: {error}"),
            Self::InvalidData(message) => write!(f, "invalid storage data: {message}"),
            Self::Chain(error) => write!(f, "stored chain validation failed: {error}"),
            Self::Genesis(error) => write!(f, "stored genesis config is invalid: {error}"),
        }
    }
}

impl Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ChainError> for StorageError {
    fn from(error: ChainError) -> Self {
        Self::Chain(error)
    }
}

impl From<GenesisConfigError> for StorageError {
    fn from(error: GenesisConfigError) -> Self {
        Self::Genesis(error)
    }
}

#[derive(Debug, Clone)]
pub struct LoadedBlockchain {
    genesis_config: GenesisConfig,
    blockchain: Blockchain,
}

impl LoadedBlockchain {
    pub fn genesis_config(&self) -> &GenesisConfig {
        &self.genesis_config
    }
    pub fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }
    pub fn into_parts(self) -> (GenesisConfig, Blockchain) {
        (self.genesis_config, self.blockchain)
    }
}

#[derive(Debug, Clone)]
struct StoredBlock {
    hash: BlockHash,
    block: Block,
    metadata: BlockMetadata,
    state: State,
}

pub fn save(
    path: impl AsRef<Path>,
    config: &GenesisConfig,
    blockchain: &Blockchain,
) -> Result<(), StorageError> {
    if blockchain.chain_id() != config.chain_id()
        || blockchain.pow_target() != config.pow_target()
        || blockchain.block(&config.genesis_hash()).is_none()
    {
        return Err(StorageError::InvalidData(
            "genesis config does not match chain",
        ));
    }

    let encoded = encode(config, blockchain);
    let path = path.as_ref();
    let temp_path = temporary_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;

    let write_result = (|| -> Result<(), StorageError> {
        file.write_all(&encoded)?;
        file.sync_all()?;
        fs::rename(&temp_path, path)?;
        if let Some(parent) = path.parent() {
            if let Ok(directory) = fs::File::open(parent) {
                let _ = directory.sync_all();
            }
        }
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

pub fn load(path: impl AsRef<Path>) -> Result<LoadedBlockchain, StorageError> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.read_to_end(&mut bytes)?;
    decode_and_validate(&bytes)
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".tmp-{}", std::process::id()));
    path.with_file_name(name)
}

fn encode(config: &GenesisConfig, chain: &Blockchain) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    put_u16(&mut out, STORAGE_VERSION);
    put_u32(&mut out, config.chain_id().value());
    put_u64(&mut out, config.timestamp().value());
    out.extend_from_slice(config.pow_target().as_bytes());
    put_u64(&mut out, config.allocations().len() as u64);
    for allocation in config.allocations() {
        put_account(
            &mut out,
            allocation.address(),
            allocation.balance(),
            allocation.nonce(),
        );
    }
    out.extend_from_slice(chain.canonical_tip_hash().as_bytes());

    let mut blocks: Vec<_> = chain.indexed_blocks().collect();
    blocks.sort_unstable_by_key(|(hash, _, _, _)| *hash.as_bytes());
    put_u64(&mut out, blocks.len() as u64);
    for (hash, block, metadata, state) in blocks {
        out.extend_from_slice(hash.as_bytes());
        put_block(&mut out, block);
        out.extend_from_slice(metadata.parent().as_bytes());
        put_u64(&mut out, metadata.height().value());
        put_u64(&mut out, metadata.cumulative_work().value());
        put_state(&mut out, state);
    }
    out
}

fn decode_and_validate(bytes: &[u8]) -> Result<LoadedBlockchain, StorageError> {
    let mut input = Decoder::new(bytes);
    if input.take(8)? != MAGIC {
        return Err(StorageError::InvalidData("wrong magic"));
    }
    if input.u16()? != STORAGE_VERSION {
        return Err(StorageError::InvalidData("unsupported version"));
    }
    let chain_id = ChainId::new(input.u32()?);
    let timestamp = BlockTimestamp::new(input.u64()?);
    let pow_target = PowTarget::from_bytes(input.array()?);
    let allocation_count = input.count()?;
    let mut allocations = Vec::new();
    for _ in 0..allocation_count {
        let (address, account) = input.account()?;
        allocations.push(GenesisAllocation::new(
            address,
            account.balance(),
            account.nonce(),
        ));
    }
    let config = GenesisConfig::new(chain_id, timestamp, pow_target, allocations)?;
    let canonical_tip = BlockHash::from_hash(Hash256::from_bytes(input.array()?));
    let block_count = input.count()?;
    if block_count == 0 {
        return Err(StorageError::InvalidData("no blocks"));
    }
    let mut records = HashMap::new();
    for _ in 0..block_count {
        let stored_hash = BlockHash::from_hash(Hash256::from_bytes(input.array()?));
        let block = input.block()?;
        if block_hash(block.header()) != stored_hash {
            return Err(StorageError::InvalidData("block hash mismatch"));
        }
        let parent = BlockHash::from_hash(Hash256::from_bytes(input.array()?));
        let metadata = BlockMetadata::new(
            parent,
            BlockHeight::new(input.u64()?),
            ChainWork::new(input.u64()?),
        );
        let state = input.state()?;
        if state.state_root() != block.header().state_root() {
            return Err(StorageError::InvalidData("stored state root mismatch"));
        }
        if records
            .insert(
                stored_hash,
                StoredBlock {
                    hash: stored_hash,
                    block,
                    metadata,
                    state,
                },
            )
            .is_some()
        {
            return Err(StorageError::InvalidData("duplicate block"));
        }
    }
    if !input.is_empty() {
        return Err(StorageError::InvalidData("trailing bytes"));
    }
    validate_and_restore(config, canonical_tip, records)
}

fn validate_and_restore(
    config: GenesisConfig,
    canonical_tip: BlockHash,
    records: HashMap<BlockHash, StoredBlock>,
) -> Result<LoadedBlockchain, StorageError> {
    if !records.contains_key(&canonical_tip) {
        return Err(StorageError::InvalidData("canonical tip missing"));
    }
    let genesis_hash = config.genesis_hash();
    let genesis = records
        .get(&genesis_hash)
        .ok_or(StorageError::InvalidData("genesis missing"))?;
    if genesis.block != config.genesis_block()
        || genesis.metadata.parent() != BlockHash::ZERO
        || genesis.metadata.height() != BlockHeight::ZERO
        || genesis.metadata.cumulative_work() != ChainWork::ZERO
        || genesis.state.state_root() != config.state_root()
    {
        return Err(StorageError::InvalidData("genesis mismatch"));
    }
    for record in records.values() {
        if record.hash != genesis_hash && !records.contains_key(&record.metadata.parent()) {
            return Err(StorageError::InvalidData("parent missing"));
        }
        if record.metadata.height() != record.block.header().height()
            || record.metadata.parent() != record.block.header().previous_block_hash()
        {
            return Err(StorageError::InvalidData("metadata mismatch"));
        }
    }

    let canonical_hashes = ancestry(&records, canonical_tip, genesis_hash)?;
    let mut chain = Blockchain::from_genesis_config(&config)?;
    for hash in canonical_hashes.iter().skip(1) {
        chain.append_block(records[hash].block.clone())?;
    }

    let mut pending: HashSet<_> = records.keys().copied().collect();
    for hash in &canonical_hashes {
        pending.remove(hash);
    }
    while !pending.is_empty() {
        let ready: Vec<_> = pending
            .iter()
            .copied()
            .filter(|hash| chain.block(&records[hash].metadata.parent()).is_some())
            .collect();
        if ready.is_empty() {
            return Err(StorageError::InvalidData("cyclic block ancestry"));
        }
        for hash in ready {
            chain.append_block(records[&hash].block.clone())?;
            pending.remove(&hash);
        }
    }
    if chain.canonical_tip_hash() != canonical_tip {
        return Err(StorageError::InvalidData("canonical tip is inconsistent"));
    }
    for (hash, record) in &records {
        if chain.metadata(hash) != Some(&record.metadata)
            || chain.state_at(hash).map(State::state_root) != Some(record.state.state_root())
        {
            return Err(StorageError::InvalidData("replayed block data mismatch"));
        }
    }
    Ok(LoadedBlockchain {
        genesis_config: config,
        blockchain: chain,
    })
}

fn ancestry(
    records: &HashMap<BlockHash, StoredBlock>,
    tip: BlockHash,
    genesis: BlockHash,
) -> Result<Vec<BlockHash>, StorageError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let mut current = tip;
    loop {
        if !seen.insert(current) {
            return Err(StorageError::InvalidData("canonical cycle"));
        }
        result.push(current);
        if current == genesis {
            break;
        }
        current = records
            .get(&current)
            .ok_or(StorageError::InvalidData("canonical block missing"))?
            .metadata
            .parent();
    }
    result.reverse();
    Ok(result)
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}
fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_account(out: &mut Vec<u8>, address: Address, balance: Amount, nonce: Nonce) {
    out.extend_from_slice(address.as_bytes());
    put_u64(out, balance.value());
    put_u64(out, nonce.as_u64());
}

fn put_state(out: &mut Vec<u8>, state: &State) {
    let mut accounts: Vec<_> = state.accounts().collect();
    accounts.sort_unstable_by_key(|(address, _)| *address.as_bytes());
    put_u64(out, accounts.len() as u64);
    for (address, account) in accounts {
        put_account(out, *address, account.balance(), account.nonce());
    }
}

fn put_block(out: &mut Vec<u8>, block: &Block) {
    let header = block.header();
    put_u16(out, header.version().value());
    put_u32(out, header.chain_id().value());
    put_u64(out, header.height().value());
    out.extend_from_slice(header.previous_block_hash().as_bytes());
    out.extend_from_slice(header.transaction_root().as_bytes());
    out.extend_from_slice(header.state_root().as_bytes());
    put_u64(out, header.timestamp().value());
    put_u64(out, header.pow_nonce().value());
    put_u64(out, block.transactions().len() as u64);
    for transaction in block.transactions() {
        let payload = transaction.payload();
        put_u16(out, payload.version().value());
        put_u32(out, payload.chain_id().value());
        out.extend_from_slice(payload.sender_public_key().as_bytes());
        out.extend_from_slice(payload.recipient().as_bytes());
        put_u64(out, payload.amount().value());
        put_u64(out, payload.nonce().as_u64());
        out.extend_from_slice(transaction.signature().as_bytes());
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
    fn take(&mut self, len: usize) -> Result<&'a [u8], StorageError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(StorageError::InvalidData("length overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(StorageError::InvalidData("truncated data"))?;
        self.offset = end;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], StorageError> {
        self.take(N)?
            .try_into()
            .map_err(|_| StorageError::InvalidData("invalid field length"))
    }
    fn u16(&mut self) -> Result<u16, StorageError> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, StorageError> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, StorageError> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    fn count(&mut self) -> Result<usize, StorageError> {
        usize::try_from(self.u64()?).map_err(|_| StorageError::InvalidData("count too large"))
    }
    fn account(&mut self) -> Result<(Address, Account), StorageError> {
        let address = Address::from_bytes(self.array()?);
        let account = Account::new(Amount::new(self.u64()?), Nonce::new(self.u64()?));
        Ok((address, account))
    }
    fn state(&mut self) -> Result<State, StorageError> {
        let count = self.count()?;
        let mut state = State::new();
        for _ in 0..count {
            let (address, account) = self.account()?;
            if state.account(&address).is_some() {
                return Err(StorageError::InvalidData("duplicate state account"));
            }
            state.set_account(address, account);
        }
        Ok(state)
    }
    fn block(&mut self) -> Result<Block, StorageError> {
        let version = BlockVersion::new(self.u16()?);
        let chain_id = ChainId::new(self.u32()?);
        let height = BlockHeight::new(self.u64()?);
        let previous = BlockHash::from_hash(Hash256::from_bytes(self.array()?));
        let merkle = MerkleRoot::from_hash(Hash256::from_bytes(self.array()?));
        let state_root = StateRoot::from_hash(Hash256::from_bytes(self.array()?));
        let timestamp = BlockTimestamp::new(self.u64()?);
        let pow_nonce = PowNonce::new(self.u64()?);
        let transaction_count = self.count()?;
        let mut transactions = Vec::new();
        for _ in 0..transaction_count {
            let payload = UnsignedTransaction::new(
                TransactionVersion::new(self.u16()?),
                ChainId::new(self.u32()?),
                PublicKeyBytes::from_bytes(self.array()?),
                Address::from_bytes(self.array()?),
                Amount::new(self.u64()?),
                Nonce::new(self.u64()?),
            );
            transactions.push(SignedTransaction::new(
                payload,
                SignatureBytes::from_bytes(self.array()?),
            ));
        }
        Ok(Block::new(
            BlockHeader::new(
                version, chain_id, height, previous, merkle, state_root, timestamp, pow_nonce,
            ),
            transactions,
        ))
    }
}
