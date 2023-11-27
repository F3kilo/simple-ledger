use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;

use clap::{arg, Parser};
use k256::ecdsa::SigningKey;
use ledger_transport::Transport;
use ledger_types::{Block, NodeInfo, Transaction, B256, Message};

/// Command line parameters of the simple-ledger node.
#[derive(Debug, Parser)]
struct Params {
    /// Name of the node.
    #[arg(short, long)]
    name: Option<String>,

    /// Socket address of the node.
    #[clap(short, long)]
    socket: SocketAddr,

    /// Socket address of another working node.
    #[clap(short, long)]
    other_node: SocketAddr,
}

fn main() {
    let params = Params::parse();

    let name = params
        .name
        .unwrap_or_else(|| names::Generator::default().next().unwrap());

    let signer = SigningKey::random(&mut rand::thread_rng());
    let address = B256::address_of(signer.verifying_key());

    let node_info = NodeInfo {
        name,
        address,
        socket: params.socket,
    };

    let node = Node::new(signer, node_info);
}

struct Node {
    info: NodeInfo,
    transport: Transport,
    signer: SigningKey,
    others: BTreeMap<B256, NodeInfo>,
    blocks: Blocks,
    pending_transactions: HashMap<B256, Transaction>,
}

impl Node {
    fn new(signer: SigningKey, info: NodeInfo) -> Self {
        let transport = Transport::new(info.socket).expect("failed to create transport");
        let others = BTreeMap::new();
        let blocks = Blocks::default();
        let pending_transactions = HashMap::new();

        Self {
            transport,
            info,
            signer,
            others,
            blocks,
            pending_transactions,
        }
    }

    pub fn run(mut self) {
        while let Some(message) = self.transport.receive() {
            self.process_message(message)
        }
    }

    fn process_message(&self, message: Message) {
        match message {
            Message::Hello(node_info) => self.process_hello(node_info),
            Message::Transaction(tx) => self.process_transaction(tx),
            Message::Block(block) => self.append_block(block),
        }
    }
}

#[derive(Debug, Default)]
struct Blocks {
    hashes: Vec<B256>,
    data: HashMap<B256, Block>,
}
