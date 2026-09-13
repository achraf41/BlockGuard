use crate::{Node, NodeError};
use blockguard_core::{
    Block, BlockHash, BlockHeight, BlockTimestamp, SignedTransaction, TransactionId,
};
use blockguard_crypto::{block_hash, transaction_id};
use blockguard_network::{
    Handshake, Message, NetworkError, NodeId, exchange_handshake, read_message, write_message,
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt, io,
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

type Writer = Arc<Mutex<TcpStream>>;
type Peers = Arc<Mutex<HashMap<u64, Writer>>>;
type KnownTransactions = Arc<Mutex<HashSet<TransactionId>>>;

#[derive(Debug)]
pub enum NetworkNodeError {
    Io(io::Error),
    Network(NetworkError),
    Node(NodeError),
    LockPoisoned,
}
impl fmt::Display for NetworkNodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "network node I/O failed: {e}"),
            Self::Network(e) => write!(f, "peer protocol failed: {e}"),
            Self::Node(e) => write!(f, "node operation failed: {e}"),
            Self::LockPoisoned => write!(f, "network node lock poisoned"),
        }
    }
}
impl Error for NetworkNodeError {}
impl From<io::Error> for NetworkNodeError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<NetworkError> for NetworkNodeError {
    fn from(e: NetworkError) -> Self {
        Self::Network(e)
    }
}
impl From<NodeError> for NetworkNodeError {
    fn from(e: NodeError) -> Self {
        Self::Node(e)
    }
}

pub struct NetworkNode {
    node: Arc<Mutex<Node>>,
    peers: Peers,
    known_transactions: KnownTransactions,
    handshake: Handshake,
    address: SocketAddr,
    running: Arc<AtomicBool>,
    next_peer: Arc<AtomicU64>,
    listener_thread: Option<thread::JoinHandle<()>>,
}

impl NetworkNode {
    pub fn bind(
        node: Node,
        address: SocketAddr,
        node_id: NodeId,
    ) -> Result<Self, NetworkNodeError> {
        let handshake = Handshake::new(
            node.genesis_config().chain_id(),
            node.genesis_config().genesis_hash(),
            node_id,
        );
        let listener = TcpListener::bind(address)?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let known_transactions = Arc::new(Mutex::new(
            node.blockchain()
                .blocks()
                .iter()
                .flat_map(|block| block.transactions())
                .map(transaction_id)
                .collect(),
        ));
        let node = Arc::new(Mutex::new(node));
        let peers = Arc::new(Mutex::new(HashMap::new()));
        let running = Arc::new(AtomicBool::new(true));
        let next_peer = Arc::new(AtomicU64::new(1));
        let (tn, tp, tr, th, ti, tk) = (
            node.clone(),
            peers.clone(),
            running.clone(),
            handshake.clone(),
            next_peer.clone(),
            known_transactions.clone(),
        );
        let listener_thread = thread::spawn(move || {
            while tr.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let id = ti.fetch_add(1, Ordering::Relaxed);
                        spawn_peer(id, stream, tn.clone(), tp.clone(), th.clone(), tk.clone());
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            node,
            peers,
            known_transactions,
            handshake,
            address,
            running,
            next_peer,
            listener_thread: Some(listener_thread),
        })
    }
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
    pub fn peer_count(&self) -> usize {
        self.peers.lock().map(|p| p.len()).unwrap_or(0)
    }
    pub fn snapshot(&self) -> Result<Node, NetworkNodeError> {
        Ok(self
            .node
            .lock()
            .map_err(|_| NetworkNodeError::LockPoisoned)?
            .clone())
    }
    pub fn connect(&self, address: SocketAddr) -> Result<(), NetworkNodeError> {
        let stream = TcpStream::connect(address)?;
        stream.set_nodelay(true)?;
        let id = self.next_peer.fetch_add(1, Ordering::Relaxed);
        setup_peer(
            id,
            stream,
            self.node.clone(),
            self.peers.clone(),
            self.handshake.clone(),
            self.known_transactions.clone(),
        )?;
        Ok(())
    }
    pub fn submit_transaction(
        &self,
        tx: SignedTransaction,
    ) -> Result<TransactionId, NetworkNodeError> {
        let id = self
            .node
            .lock()
            .map_err(|_| NetworkNodeError::LockPoisoned)?
            .submit_transaction(tx.clone())?;
        self.known_transactions
            .lock()
            .map_err(|_| NetworkNodeError::LockPoisoned)?
            .insert(id);
        broadcast(&self.peers, None, &Message::Transaction(tx));
        Ok(id)
    }
    pub fn produce_block(
        &self,
        timestamp: BlockTimestamp,
        max: usize,
    ) -> Result<Block, NetworkNodeError> {
        let block = self
            .node
            .lock()
            .map_err(|_| NetworkNodeError::LockPoisoned)?
            .produce_block(timestamp, max)?;
        broadcast(&self.peers, None, &Message::Block(block.clone()));
        Ok(block)
    }
    pub fn accept_block(&self, block: Block) -> Result<BlockHash, NetworkNodeError> {
        let hash = self
            .node
            .lock()
            .map_err(|_| NetworkNodeError::LockPoisoned)?
            .accept_block(block.clone())?;
        broadcast(&self.peers, None, &Message::Block(block));
        Ok(hash)
    }
    pub fn send_to_all(&self, message: &Message) {
        broadcast(&self.peers, None, message)
    }
}
impl Drop for NetworkNode {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Ok(mut peers) = self.peers.lock() {
            for writer in peers.values() {
                if let Ok(stream) = writer.lock() {
                    let _ = stream.shutdown(Shutdown::Both);
                }
            }
            peers.clear();
        }
        if let Some(h) = self.listener_thread.take() {
            let _ = h.join();
        }
    }
}

fn spawn_peer(
    id: u64,
    stream: TcpStream,
    node: Arc<Mutex<Node>>,
    peers: Peers,
    handshake: Handshake,
    known_transactions: KnownTransactions,
) {
    thread::spawn(move || {
        let _ = setup_peer(id, stream, node, peers, handshake, known_transactions);
    });
}
fn setup_peer(
    id: u64,
    mut stream: TcpStream,
    node: Arc<Mutex<Node>>,
    peers: Peers,
    handshake: Handshake,
    known_transactions: KnownTransactions,
) -> Result<(), NetworkNodeError> {
    stream.set_nodelay(true)?;
    exchange_handshake(&mut stream, &handshake)?;
    let reader = stream.try_clone()?;
    let writer = Arc::new(Mutex::new(stream));
    peers
        .lock()
        .map_err(|_| NetworkNodeError::LockPoisoned)?
        .insert(id, writer.clone());
    send(&writer, &Message::GetTip);
    let rp = peers.clone();
    thread::spawn(move || peer_loop(id, reader, node, rp, writer, known_transactions));
    Ok(())
}
fn peer_loop(
    id: u64,
    mut reader: TcpStream,
    node: Arc<Mutex<Node>>,
    peers: Peers,
    writer: Writer,
    known_transactions: KnownTransactions,
) {
    while let Ok(message) = read_message(&mut reader) {
        handle_message(id, message, &node, &peers, &writer, &known_transactions)
    }
    if let Ok(mut p) = peers.lock() {
        p.remove(&id);
    }
}
fn handle_message(
    source: u64,
    message: Message,
    node: &Arc<Mutex<Node>>,
    peers: &Peers,
    writer: &Writer,
    known_transactions: &KnownTransactions,
) {
    match message {
        Message::Transaction(tx) => {
            let id = transaction_id(&tx);
            if known_transactions
                .lock()
                .map(|known| known.contains(&id))
                .unwrap_or(true)
            {
                return;
            }
            let accepted = node
                .lock()
                .ok()
                .and_then(|mut n| {
                    if n.mempool().contains(&id) {
                        None
                    } else {
                        n.submit_transaction(tx.clone()).ok()
                    }
                })
                .is_some();
            if accepted {
                if let Ok(mut known) = known_transactions.lock() {
                    known.insert(id);
                }
                broadcast(peers, Some(source), &Message::Transaction(tx));
            }
        }
        Message::Block(block) => {
            let hash = block_hash(block.header());
            let accepted = node
                .lock()
                .ok()
                .and_then(|mut n| {
                    if n.blockchain().block(&hash).is_some() {
                        None
                    } else {
                        n.accept_block(block.clone()).ok()
                    }
                })
                .is_some();
            if accepted {
                broadcast(peers, Some(source), &Message::Block(block));
            }
        }
        Message::Ping(n) => send(writer, &Message::Pong(n)),
        Message::Pong(_) => {}
        Message::GetTip => {
            if let Ok(n) = node.lock() {
                send(
                    writer,
                    &Message::Tip {
                        height: n.blockchain().tip().header().height(),
                        block_hash: n.blockchain().canonical_tip_hash(),
                    },
                );
            }
        }
        Message::Tip { height, block_hash } => {
            if let Ok(n) = node.lock() {
                let local = n.blockchain().tip().header().height();
                if height > local && n.blockchain().block(&block_hash).is_none() {
                    send(
                        writer,
                        &Message::GetBlocks {
                            start_height: BlockHeight::new(local.value() + 1),
                        },
                    );
                }
            }
        }
        Message::GetBlocks { start_height } => {
            if let Ok(n) = node.lock() {
                for block in n
                    .blockchain()
                    .blocks()
                    .iter()
                    .skip(start_height.value() as usize)
                {
                    send(writer, &Message::Block(block.clone()));
                }
            }
        }
        Message::Handshake(_) => {}
    }
}
fn send(writer: &Writer, message: &Message) {
    if let Ok(mut stream) = writer.lock() {
        let _ = write_message(&mut *stream, message);
    }
}
fn broadcast(peers: &Peers, exclude: Option<u64>, message: &Message) {
    if let Ok(peers) = peers.lock() {
        for (id, writer) in peers.iter() {
            if Some(*id) != exclude {
                send(writer, message)
            }
        }
    }
}
