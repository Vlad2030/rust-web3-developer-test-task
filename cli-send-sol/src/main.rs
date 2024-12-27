// sorry for shitcode lol

use chrono;
use serde;
use serde_yaml;
use solana_client;
use solana_sdk;
use std;
use std::str::FromStr;
use tokio;
use solana_sdk::signature::Signer;

#[derive(serde::Deserialize)]
struct Config {
    wallets: Vec<String>,
    recipients: Vec<String>,
    cluster: String,
    amount: u64,
}

#[derive(serde::Serialize)]
struct TransactionStatus {
    wallet: String,
    recipient: String,
    hash: String,
    status: String,
    duration: f64,
}

#[tokio::main]
async fn main() {
    let config: Config = load_config("config.yaml");

    let rpc_url = match config.cluster.as_str() {
        "mainnet" => "https://api.mainnet-beta.solana.com",
        "testnet" => "https://api.testnet.solana.com",
        "devnet" => "https://api.devnet.solana.com",
        _ => panic!("Unsupported cluster: {}", config.cluster),
    };

    let client = solana_client::rpc_client::RpcClient::new(rpc_url.to_string());
    let mut transaction_statuses = Vec::new();

    for (wallet, recipient) in config.wallets.iter().zip(config.recipients.iter()) {
        let recipient_pubkey = solana_sdk::pubkey::Pubkey::from_str(recipient).expect("Invalid recipient address");
        let keypair = parse_private_key(wallet);

        let start_time = chrono::Utc::now();
        let signature = send_transaction(&client, &keypair, &recipient_pubkey, config.amount).await;
        let end_time = chrono::Utc::now();
        let duration = (end_time - start_time).num_milliseconds() as f64 / 1000.0;

        let status = check_transaction_status(&client, &signature).await;

        transaction_statuses.push(TransactionStatus {
            wallet: wallet.clone(),
            recipient: recipient.clone(),
            hash: signature.to_string(),
            status,
            duration,
        });
    }

    print_transaction_statuses(&transaction_statuses);
}

fn load_config(path: &str) -> Config {
    let content = std::fs::read_to_string(path).expect("Failed to read config file");
    serde_yaml::from_str(&content).expect("Invalid config format")
}

fn parse_private_key(private_key: &str) -> solana_sdk::signature::Keypair {
    let keypair = solana_sdk::signature::Keypair::from_base58_string(private_key);
    // keypair.expect("Invalid private key format")
    keypair
}

async fn send_transaction(client: &solana_client::rpc_client::RpcClient, sender: &solana_sdk::signature::Keypair, recipient: &solana_sdk::pubkey::Pubkey, amount: u64) -> solana_sdk::signature::Signature {
    let recent_blockhash = client.get_latest_blockhash().expect("Failed to get blockhash");

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[
            solana_sdk::system_instruction::transfer(
                &sender.pubkey(),
                recipient,
                amount,
            )
        ],
        Some(&sender.pubkey()),
        &[sender],
        recent_blockhash,
    );

    client.send_and_confirm_transaction(&tx).expect("Failed to send transaction")
}

async fn check_transaction_status(client: &solana_client::rpc_client::RpcClient, signature: &solana_sdk::signature::Signature) -> String {
    match client.confirm_transaction(signature) {
        Ok(true) => "Confirmed".to_string(),
        Ok(false) => "Unconfirmed".to_string(),
        Err(_) => "Error".to_string(),
    }
}

fn print_transaction_statuses(statuses: &[TransactionStatus]) {
    for status in statuses {
        println!(
            "Wallet: {}, Recipient: {}, Hash: {}, Status: {}, Duration: {:.2} sec",
            status.wallet, status.recipient, status.hash, status.status, status.duration
        );
    }
}
