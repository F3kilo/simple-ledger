use std::net::SocketAddr;

use k256::ecdsa::{RecoveryId, Signature as K256Signature, SigningKey, VerifyingKey};
use k256::elliptic_curve::bigint::CheckedSub;
use k256::elliptic_curve::consts::U32;
use k256::elliptic_curve::generic_array::GenericArray;
use k256::schnorr::signature::hazmat::PrehashSigner;
use k256::sha2::Digest;
use k256::U256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub prev_hash: B256,
    pub number: u64,
    pub transactions: Vec<Transaction>,
}

impl BlockData {
    pub fn hash(&self) -> B256 {
        let mut hasher = k256::sha2::Sha256::new();
        hasher.update(self.prev_hash.0);

        for tx in &self.transactions {
            hasher.update(tx.hash.0);
        }

        let result = hasher.finalize();
        B256(result.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub hash: B256,
    pub data: BlockData,
    pub proposer: B256,
    pub signature: Signature,
}

impl Block {
    /// Creates a new signed block.
    pub fn new(data: BlockData, signer: &SigningKey) -> Self {
        let hash = data.hash();
        let signature = Signature::sign(signer, hash);
        let proposer = B256::address_of(signer.verifying_key());

        Self {
            hash,
            data,
            proposer,
            signature,
        }
    }

    /// Check correctness of block signature.
    pub fn verify(&self) -> Option<()> {
        let expected_hash = self.data.hash();
        if self.hash != expected_hash {
            return None;
        }

        let expectet_proposer = self.signature.recover(expected_hash)?;
        if self.proposer != expectet_proposer {
            return None;
        }

        Some(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub to: B256,
    pub amount: u64,
}

impl TransactionData {
    pub fn hash(&self) -> B256 {
        let mut hasher = k256::sha2::Sha256::new();
        hasher.update(self.to.0);
        hasher.update(self.amount.to_be_bytes());
        let result = hasher.finalize();
        B256(result.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: B256,
    pub from: B256,
    pub data: TransactionData,
    pub signature: Signature,
}

impl Transaction {
    pub fn new(data: TransactionData, signer: &SigningKey) -> Self {
        let hash = data.hash();
        let signature = Signature::sign(signer, hash);
        let from = B256::address_of(signer.verifying_key());
        Self {
            hash,
            from,
            data,
            signature,
        }
    }

    /// Check correctness of transaction signature.
    pub fn verify(&self) -> Option<()> {
        let expected_hash = self.data.hash();
        if self.hash != expected_hash {
            return None;
        }

        let expected_from = self.signature.recover(expected_hash)?;
        if self.from != expected_from {
            return None;
        }

        Some(())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct B256(pub [u8; 32]);

impl std::fmt::Debug for B256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "B256({})", hex::encode(self.0))
    }
}

impl std::fmt::Display for B256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl B256 {
    pub fn hash_of(data: impl AsRef<[u8]>) -> Self {
        let mut hasher = k256::sha2::Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self(result.into())
    }

    pub fn address_of(key: &VerifyingKey) -> Self {
        let encoded_point = &key.to_encoded_point(false);
        let data = encoded_point.as_bytes();
        Self::hash_of(data)
    }

    pub fn distance(&self, other: B256) -> U256 {
        let self_num = U256::from_be_slice(&self.0);
        let other_num = U256::from_be_slice(&other.0);
        if self_num > other_num {
            self_num.checked_sub(&other_num).unwrap()
        } else {
            other_num.checked_sub(&self_num).unwrap()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub address: B256,
    pub socket: SocketAddr,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub r: B256,
    pub s: B256,
    pub recovery_id: u8,
}
impl Signature {
    fn sign(signer: &SigningKey, hash: B256) -> Self {
        let (sig, ver) = signer
            .sign_prehash(&hash.0)
            .expect("prehash should be signed");

        Self {
            r: B256(sig.r().to_bytes().try_into().unwrap()),
            s: B256(sig.s().to_bytes().try_into().unwrap()),
            recovery_id: ver.to_byte(),
        }
    }

    pub fn recover(&self, hash: B256) -> Option<B256> {
        let (recoverable_sig, recovery_id) = self.as_signature();
        let verify_key =
            VerifyingKey::recover_from_prehash(&hash.0, &recoverable_sig, recovery_id).ok()?;

        let address = B256::address_of(&verify_key);
        Some(address)
    }

    /// Checks if the signature is created by the `address`.
    pub fn verify(&self, hash: B256, address: B256) -> Option<()> {
        let recovered = self.recover(hash)?;
        (recovered == address).then_some(())
    }

    /// Retrieves the recovery signature.
    fn as_signature(&self) -> (K256Signature, RecoveryId) {
        let recovery_id = RecoveryId::from_byte(self.recovery_id).unwrap();
        let r: &GenericArray<u8, U32> = GenericArray::from_slice(&self.r.0);
        let s: &GenericArray<u8, U32> = GenericArray::from_slice(&self.s.0);
        let sig = K256Signature::from_scalars(*r, *s).unwrap();
        (sig, recovery_id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Hello(NodeInfo),
    Transaction(Transaction),
    Block(Block),
    SyncBlock(B256, u64),
}

#[cfg(test)]
mod tests {
    use k256::ecdsa::SigningKey;

    use crate::{Signature, B256};

    #[test]
    fn sign_and_verify() {
        let signer = SigningKey::from_slice(&[42; 32]).unwrap();

        let hash = B256::default();
        let signature = Signature::sign(&signer, hash);
        signature
            .verify(hash, B256::address_of(signer.verifying_key()))
            .unwrap();

        assert!(signature.verify(hash, B256::default()).is_none());
    }
}
