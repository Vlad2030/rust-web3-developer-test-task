use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{Instruction, AccountMeta},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use borsh::{BorshSerialize, BorshDeserialize};
use std::str::FromStr;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ProgramInstructionData {
    pub instruction_type: u64,
    pub amount: u64,
}

#[tokio::main]
async fn main() {
    let program_id = Pubkey::from_str("7R8EqmnNg5jycRPkH6pr5Mxk2yAjghJiuq4oDnxCRxLb").unwrap();

    let rpc_url = String::from("https://api.devnet.solana.com");
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let payer = Keypair::from_base58_string("");

    let data = ProgramInstructionData{instruction_type: 1, amount: 0};

    let instruction = Instruction::new_with_borsh(
        program_id,
        &data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(Pubkey::new_unique(), false),
        ],
    );

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], client.get_latest_blockhash().unwrap());

    match client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => println!("Transaction Signature: {}", signature),
        Err(err) => eprintln!("Error sending transaction: {}", err),
    }
}
