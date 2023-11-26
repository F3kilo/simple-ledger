use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;

use clap::{arg, Parser};
use k256::ecdsa::SigningKey;
use ledger_types::{Block, NodeInfo, Transaction, B256};

#[derive(Debug, Parser)]
struct Params {
    #[arg(short, long)]
    name: Option<String>,

    #[clap(short, long)]
    socket: SocketAddr,
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
    signer: SigningKey,
    others: BTreeMap<B256, NodeInfo>,
    blocks: Blocks,
    pending_transactions: Vec<Transaction>,
}

impl Node {
    fn new(signer: SigningKey, info: NodeInfo) -> Self {
        let others = BTreeMap::new();
        let blocks = Blocks::default();
        let pending_transactions = Vec::new();

        Self {
            info,
            signer,
            others,
            blocks,
            pending_transactions,
        }
    }
}

#[derive(Debug, Default)]
struct Blocks {
    hashes: Vec<B256>,
    data: HashMap<B256, Block>,
}
