use std::net::SocketAddr;

use clap::Parser;
use k256::ecdsa::SigningKey;
use ledger_transport::Transport;
use ledger_types::{Message, Transaction, TransactionData, B256};

/// Command line parameters of the simple-ledger node.
#[derive(Debug, Parser)]
struct Params {
    /// Socket address of the client.
    #[clap(short, long)]
    socket: Option<SocketAddr>,

    /// Hex representation of a signing key.
    #[clap(short, long)]
    key: Option<String>,

    /// Socket address of the node to communicate.
    #[clap(short, long)]
    node: Option<SocketAddr>,

    /// Perform transfer.
    #[clap(short, long)]
    transfer_to: Option<String>,

    /// Amount to transfer.
    #[clap(short, long)]
    amount: Option<u64>,

    /// Get balance.
    #[clap(short, long)]
    balance: bool,

    /// Generate a new account.
    #[clap(short, long)]
    crate_account: bool,
}

fn main() {
    let params = Params::parse();

    if params.crate_account {
        let key = SigningKey::random(&mut rand::thread_rng());
        let hex_repr = hex::encode(key.to_bytes().as_slice());
        println!("Generated key: {}", hex_repr);
    };

    if params.balance {
        let socket = params.socket.expect("client socket should be specified");
        let key = params.key.expect("client key should be specified");
        let node_socket = params.node.expect("node socket should be specified");

        let key_bytes = hex::decode(key).expect("client key should be a valid hex string");
        let signer = SigningKey::from_bytes(key_bytes.as_slice().into()).unwrap();
        let address = B256::address_of(signer.verifying_key());
        println!("Address: {}", address);

        let transport = Transport::new(socket).expect("client transport should be initialized");
        transport
            .send(node_socket, &Message::BalanceOf(socket, address))
            .expect("balance request should be sent");
        let balance = transport
            .receive::<u64>()
            .expect("balance response should be received");
        println!("Balance: {}", balance);
        return;
    }

    if let Some(to) = params.transfer_to {
        let socket = params.socket.expect("client socket should be specified");
        let key = params.key.expect("client key should be specified");
        let node_socket = params.node.expect("node socket should be specified");
        let amount = params.amount.expect("transfer amount should be specified");

        let key_bytes = hex::decode(key).expect("client key should be a valid hex string");
        let signer = SigningKey::from_bytes(key_bytes.as_slice().into()).unwrap();
        let address = B256::address_of(signer.verifying_key());
        println!("Address: {}", address);

        let transport = Transport::new(socket).expect("client transport should be initialized");
        let to = B256::from_hex_string(&to).unwrap();
        let data = TransactionData { to, amount };
        let transaction = Transaction::new(data, &signer);
        transport
            .send(node_socket, &Message::Transaction(transaction))
            .expect("transaction request should be sent");
    }
}
