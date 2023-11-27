use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;

use clap::{arg, Parser};
use k256::ecdsa::SigningKey;
use ledger_transport::Transport;
use ledger_types::{Block, BlockData, Message, NodeInfo, Transaction, B256};

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

    println!(
        "Creating Node {} with socket {}",
        node_info.name, node_info.socket
    );
    let node = Node::new(signer, node_info.clone());
    node.run();
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

    fn process_message(&mut self, message: Message) {
        match message {
            Message::Hello(node_info) => self.process_hello(node_info),
            Message::Transaction(tx) => self.process_transaction(tx),
            Message::Block(block) => self.process_block(block),
            Message::SyncBlock(sender, start) => self.process_sync_block(sender, start),
            Message::BalanceOf(sender, address) => self.process_balance_of(sender, address),
        }
    }

    fn process_hello(&mut self, node_info: NodeInfo) {
        let replaced = self.others.insert(node_info.address, node_info.clone());

        // If the node is new for us, let's say hi to it.
        if replaced.is_none() && node_info.address != self.info.address {
            self.send_to_others(Message::Hello(node_info));
        }
    }

    fn process_transaction(&mut self, tx: Transaction) {
        if tx.verify().is_none() {
            return;
        }

        if self.blocks.balance_of(tx.from) < tx.data.amount {
            return;
        }

        let replaced = self.pending_transactions.insert(tx.hash, tx.clone());

        // If the transaction is new for us, let's broadcast it.
        if replaced.is_none() {
            self.send_to_others(Message::Transaction(tx));
            self.propose_block();
        }
    }

    fn process_block(&mut self, block: Block) {
        if block.verify().is_none() || block.proposer == self.info.address {
            return;
        }

        let block_append_result = self.blocks.append(block.clone());

        // If the block is new for us, let's broadcast it.
        match block_append_result {
            BlockAppendResult::NeedSync(start) => {
                self.send_to_others(Message::SyncBlock(self.info.address, start))
            }
            BlockAppendResult::Added => self.send_to_others(Message::Block(block)),
            BlockAppendResult::None => todo!(),
        }
    }

    fn process_sync_block(&mut self, sender: B256, start: u64) {
        let Some(sender_info) = self.others.get(&sender) else {
            return;
        };

        for i in start..self.blocks.hashes.len() as u64 {
            let block = self.blocks.data_by_number(i).unwrap();
            self.transport.send(sender_info.socket, block);
        }
    }

    fn send_to_others(&self, msg: Message) {
        for other in self.others.values() {
            self.transport.send(other.socket, &msg);
        }
    }

    fn propose_block(&mut self) {
        let transactions = self.pending_transactions.drain();

        let block = Block::new(
            BlockData {
                prev_hash: *self.blocks.hashes.last().unwrap(),
                number: self.blocks.hashes.len() as u64,
                transactions: transactions.map(|(_, tx)| tx).collect(),
            },
            &self.signer,
        );

        self.blocks.append_unchecked(block.clone());
        self.send_to_others(Message::Block(block));
    }

    fn process_balance_of(&self, sender: SocketAddr, address: B256) {
        let balance = self.blocks.balance_of(address);
        self.transport.send(sender, &balance);
    }
}

#[derive(Debug, Default)]
struct Blocks {
    hashes: Vec<B256>,
    data: HashMap<B256, Block>,
}
impl Blocks {
    pub fn append(&mut self, block: Block) -> BlockAppendResult {
        let new_block_number = block.data.number;
        if self.hashes.is_empty() && new_block_number == 0 {
            self.append_unchecked(block);
            return BlockAppendResult::Added;
        }

        if new_block_number == 0 {
            return BlockAppendResult::None;
        }

        let prev_block_hash = self.hashes[new_block_number as usize - 1];
        if block.data.prev_hash != prev_block_hash {
            return BlockAppendResult::None;
        }

        let next_block_number = self.hashes.len() as u64;
        match new_block_number.cmp(&next_block_number) {
            Ordering::Equal => {
                self.append_unchecked(block);
                BlockAppendResult::Added
            }
            Ordering::Greater => BlockAppendResult::NeedSync(next_block_number),
            Ordering::Less => {
                let current_hash = self.hashes[new_block_number as usize - 1];
                let current_block = &self.data[&current_hash];

                let current_distance = current_block.proposer.distance(prev_block_hash);
                let new_distance = block.proposer.distance(prev_block_hash);
                if current_distance > new_distance {
                    self.hashes.truncate(new_block_number as usize);
                    self.append_unchecked(block);
                    return BlockAppendResult::NeedSync(new_block_number + 1);
                }

                BlockAppendResult::None
            }
        }
    }

    fn append_unchecked(&mut self, block: Block) {
        self.hashes.push(block.hash);
        self.data.insert(block.hash, block);
    }

    pub fn data_by_number(&self, number: u64) -> Option<&Block> {
        let hash = self.hashes.get(number as usize)?;
        self.data.get(hash)
    }

    fn balance_of(&self, address: B256) -> u64 {
        let transactions_iter = self
            .hashes
            .iter()
            .flat_map(|hash| &self.data[hash].data.transactions);
        let mut balance = 0;
        for transaction in transactions_iter {
            if transaction.data.to == address {
                balance += transaction.data.amount;
            }
            if transaction.from == address {
                balance = balance.saturating_sub(transaction.data.amount);
            }
        }
        balance
    }
}

#[derive(Debug)]
pub enum BlockAppendResult {
    NeedSync(u64),
    Added,
    None,
}
