use std::cmp::Ordering;
use std::collections::HashMap;

use ledger_types::{Block, B256};

#[derive(Debug, Default)]
pub struct Blocks {
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
