use blockguard_core::{
    Address, Amount, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion,
    ChainId, Hash256, MerkleRoot, Nonce, PowNonce, PowTarget, PublicKeyBytes, SignatureBytes,
    SignedTransaction, StateRoot, TransactionVersion, UnsignedTransaction,
};
use std::{
    error::Error,
    fmt,
    io::{self, Read, Write},
    net::TcpStream,
};

pub const MAGIC: [u8; 4] = *b"BGNW";
pub const PROTOCOL_VERSION: u16 = 2;
pub const MAX_FRAME_SIZE: usize = 1024 * 1024;
pub const MAX_BLOCK_TRANSACTIONS: usize = 4096;
pub type NodeId = [u8; 16];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {
    pub protocol_version: u16,
    pub chain_id: ChainId,
    pub genesis_hash: BlockHash,
    pub node_id: NodeId,
}
impl Handshake {
    pub fn new(chain_id: ChainId, genesis_hash: BlockHash, node_id: NodeId) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            chain_id,
            genesis_hash,
            node_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Handshake(Handshake),
    Transaction(SignedTransaction),
    Block(Block),
    Ping(u64),
    Pong(u64),
    GetTip,
    Tip {
        height: BlockHeight,
        block_hash: BlockHash,
    },
    GetBlocks {
        start_height: BlockHeight,
    },
}

#[derive(Debug)]
pub enum NetworkError {
    Io(io::Error),
    WrongMagic,
    UnsupportedVersion(u16),
    UnknownMessageType(u8),
    OversizedFrame(usize),
    TruncatedFrame,
    Malformed(&'static str),
    HandshakeRequired,
    WrongChainId,
    DifferentGenesis,
}
impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "network I/O failed: {e}"),
            Self::WrongMagic => write!(f, "wrong network magic"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported protocol version {v}"),
            Self::UnknownMessageType(t) => write!(f, "unknown message type {t}"),
            Self::OversizedFrame(n) => write!(f, "frame payload {n} exceeds limit"),
            Self::TruncatedFrame => write!(f, "truncated frame"),
            Self::Malformed(m) => write!(f, "malformed message: {m}"),
            Self::HandshakeRequired => write!(f, "expected handshake"),
            Self::WrongChainId => write!(f, "peer uses a different chain ID"),
            Self::DifferentGenesis => write!(f, "peer uses a different genesis block"),
        }
    }
}
impl Error for NetworkError {}
impl From<io::Error> for NetworkError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

pub fn encode_frame(message: &Message) -> Result<Vec<u8>, NetworkError> {
    let (kind, payload) = encode_payload(message);
    if payload.len() > MAX_FRAME_SIZE {
        return Err(NetworkError::OversizedFrame(payload.len()));
    }
    let mut out = Vec::with_capacity(11 + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}
pub fn write_message(mut writer: impl Write, message: &Message) -> Result<(), NetworkError> {
    writer.write_all(&encode_frame(message)?)?;
    Ok(())
}
pub fn read_message(mut reader: impl Read) -> Result<Message, NetworkError> {
    let mut header = [0; 11];
    read_exact_frame(&mut reader, &mut header)?;
    if header[..4] != MAGIC {
        return Err(NetworkError::WrongMagic);
    }
    let version = u16::from_be_bytes([header[4], header[5]]);
    if version != PROTOCOL_VERSION {
        return Err(NetworkError::UnsupportedVersion(version));
    }
    let kind = header[6];
    if !(1..=8).contains(&kind) {
        return Err(NetworkError::UnknownMessageType(kind));
    }
    let len = u32::from_be_bytes(header[7..11].try_into().unwrap()) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(NetworkError::OversizedFrame(len));
    }
    let mut payload = vec![0; len];
    read_exact_frame(&mut reader, &mut payload)?;
    decode_payload(kind, &payload)
}
fn read_exact_frame(reader: &mut impl Read, bytes: &mut [u8]) -> Result<(), NetworkError> {
    reader.read_exact(bytes).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            NetworkError::TruncatedFrame
        } else {
            NetworkError::Io(e)
        }
    })
}

pub fn exchange_handshake(
    stream: &mut TcpStream,
    local: &Handshake,
) -> Result<Handshake, NetworkError> {
    write_message(&mut *stream, &Message::Handshake(local.clone()))?;
    stream.flush()?;
    let Message::Handshake(remote) = read_message(&mut *stream)? else {
        return Err(NetworkError::HandshakeRequired);
    };
    validate_handshake(local, &remote)?;
    Ok(remote)
}
pub fn validate_handshake(local: &Handshake, remote: &Handshake) -> Result<(), NetworkError> {
    if remote.protocol_version != PROTOCOL_VERSION {
        return Err(NetworkError::UnsupportedVersion(remote.protocol_version));
    }
    if remote.chain_id != local.chain_id {
        return Err(NetworkError::WrongChainId);
    }
    if remote.genesis_hash != local.genesis_hash {
        return Err(NetworkError::DifferentGenesis);
    }
    Ok(())
}

fn encode_payload(message: &Message) -> (u8, Vec<u8>) {
    let mut o = Vec::new();
    match message {
        Message::Handshake(h) => {
            put_u16(&mut o, h.protocol_version);
            put_u32(&mut o, h.chain_id.value());
            o.extend_from_slice(h.genesis_hash.as_bytes());
            o.extend_from_slice(&h.node_id);
            (1, o)
        }
        Message::Transaction(tx) => {
            put_tx(&mut o, tx);
            (2, o)
        }
        Message::Block(b) => {
            put_block(&mut o, b);
            (3, o)
        }
        Message::Ping(n) => {
            put_u64(&mut o, *n);
            (4, o)
        }
        Message::Pong(n) => {
            put_u64(&mut o, *n);
            (5, o)
        }
        Message::GetTip => (6, o),
        Message::Tip { height, block_hash } => {
            put_u64(&mut o, height.value());
            o.extend_from_slice(block_hash.as_bytes());
            (7, o)
        }
        Message::GetBlocks { start_height } => {
            put_u64(&mut o, start_height.value());
            (8, o)
        }
    }
}
fn decode_payload(kind: u8, payload: &[u8]) -> Result<Message, NetworkError> {
    let mut d = Decoder::new(payload);
    let m = match kind {
        1 => Message::Handshake(Handshake {
            protocol_version: d.u16()?,
            chain_id: ChainId::new(d.u32()?),
            genesis_hash: BlockHash::from_hash(Hash256::from_bytes(d.array()?)),
            node_id: d.array()?,
        }),
        2 => Message::Transaction(d.tx()?),
        3 => Message::Block(d.block()?),
        4 => Message::Ping(d.u64()?),
        5 => Message::Pong(d.u64()?),
        6 => Message::GetTip,
        7 => Message::Tip {
            height: BlockHeight::new(d.u64()?),
            block_hash: BlockHash::from_hash(Hash256::from_bytes(d.array()?)),
        },
        8 => Message::GetBlocks {
            start_height: BlockHeight::new(d.u64()?),
        },
        _ => return Err(NetworkError::UnknownMessageType(kind)),
    };
    if !d.empty() {
        return Err(NetworkError::Malformed("trailing payload bytes"));
    }
    Ok(m)
}

fn put_tx(o: &mut Vec<u8>, t: &SignedTransaction) {
    put_u16(o, t.payload().version().value());
    put_u32(o, t.payload().chain_id().value());
    o.extend_from_slice(t.payload().sender_public_key().as_bytes());
    o.extend_from_slice(t.payload().recipient().as_bytes());
    put_u64(o, t.payload().amount().value());
    put_u64(o, t.payload().nonce().as_u64());
    o.extend_from_slice(t.signature().as_bytes());
}
fn put_block(o: &mut Vec<u8>, b: &Block) {
    let h = b.header();
    put_u16(o, h.version().value());
    put_u32(o, h.chain_id().value());
    put_u64(o, h.height().value());
    o.extend_from_slice(h.previous_block_hash().as_bytes());
    o.extend_from_slice(h.transaction_root().as_bytes());
    o.extend_from_slice(h.state_root().as_bytes());
    o.extend_from_slice(h.pow_target().as_bytes());
    put_u64(o, h.timestamp().value());
    put_u64(o, h.pow_nonce().value());
    put_u32(o, b.transaction_count() as u32);
    for t in b.transactions() {
        put_tx(o, t)
    }
}
fn put_u16(o: &mut Vec<u8>, v: u16) {
    o.extend_from_slice(&v.to_be_bytes())
}
fn put_u32(o: &mut Vec<u8>, v: u32) {
    o.extend_from_slice(&v.to_be_bytes())
}
fn put_u64(o: &mut Vec<u8>, v: u64) {
    o.extend_from_slice(&v.to_be_bytes())
}
struct Decoder<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Decoder<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, p: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], NetworkError> {
        let e = self
            .p
            .checked_add(n)
            .ok_or(NetworkError::Malformed("length overflow"))?;
        let x = self
            .b
            .get(self.p..e)
            .ok_or(NetworkError::Malformed("truncated payload"))?;
        self.p = e;
        Ok(x)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], NetworkError> {
        Ok(self.take(N)?.try_into().unwrap())
    }
    fn u16(&mut self) -> Result<u16, NetworkError> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, NetworkError> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, NetworkError> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    fn empty(&self) -> bool {
        self.p == self.b.len()
    }
    fn tx(&mut self) -> Result<SignedTransaction, NetworkError> {
        let u = UnsignedTransaction::new(
            TransactionVersion::new(self.u16()?),
            ChainId::new(self.u32()?),
            PublicKeyBytes::from_bytes(self.array()?),
            Address::from_bytes(self.array()?),
            Amount::new(self.u64()?),
            Nonce::new(self.u64()?),
        );
        Ok(SignedTransaction::new(
            u,
            SignatureBytes::from_bytes(self.array()?),
        ))
    }
    fn block(&mut self) -> Result<Block, NetworkError> {
        let h = BlockHeader::new(
            BlockVersion::new(self.u16()?),
            ChainId::new(self.u32()?),
            BlockHeight::new(self.u64()?),
            BlockHash::from_hash(Hash256::from_bytes(self.array()?)),
            MerkleRoot::from_hash(Hash256::from_bytes(self.array()?)),
            StateRoot::from_hash(Hash256::from_bytes(self.array()?)),
            PowTarget::from_bytes(self.array()?),
            BlockTimestamp::new(self.u64()?),
            PowNonce::new(self.u64()?),
        );
        let n = self.u32()? as usize;
        if n > MAX_BLOCK_TRANSACTIONS {
            return Err(NetworkError::Malformed("too many block transactions"));
        }
        let tx_len = 139usize;
        if n > self.b.len().saturating_sub(self.p) / tx_len {
            return Err(NetworkError::Malformed("truncated block transactions"));
        }
        let mut ts = Vec::with_capacity(n);
        for _ in 0..n {
            ts.push(self.tx()?)
        }
        Ok(Block::new(h, ts))
    }
}
