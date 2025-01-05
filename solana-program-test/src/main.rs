use {
    borsh::{BorshDeserialize, BorshSerialize},
    dotenv::dotenv,
    lazy_static::lazy_static,
    solana_client::{client_error::Result, rpc_client::RpcClient},
    solana_program::system_instruction,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    std::str::FromStr,
};

fn main() {}

#[derive(BorshSerialize, BorshDeserialize)]
struct ProgramData {
    code: u8,
    amount: u64,
}

lazy_static! {
    static ref RPC_URL: String = {
        let cluster = std::env::var("CLUSTER").unwrap_or("devnet".to_string());

        match cluster.as_str() {
            "mainnet" => "https://api.mainnet-beta.solana.com".into(),
            "devnet" => "https://api.devnet.solana.com".into(),
            "testnet" => "https://api.testnet.solana.com".into(),
            _ => "https://api.devnet.solana.com".into(),
        }
    };
    static ref PROGRAM_ID: String = {
        dotenv().ok();
        std::env::var("PROGRAM_ID").unwrap()
    };
    static ref KEYPAIR: String = {
        dotenv().ok();
        std::env::var("KEYPAIR").unwrap()
    };
}

#[cfg(test)]
mod client_tests {
    use super::*;

    #[test]
    fn test_balance() {
        let program_id = Pubkey::from_str(PROGRAM_ID.as_str()).unwrap();

        let client =
            RpcClient::new_with_commitment(RPC_URL.as_str(), CommitmentConfig::confirmed());

        let payer = Keypair::from_base58_string(KEYPAIR.as_str());
        println!("payer: {}", payer.pubkey());

        let (receiver, bump_seed) = solana_program::pubkey::Pubkey::find_program_address(
            &[payer.pubkey().as_ref()],
            &program_id,
        );
        println!("receiver: {}", receiver);

        let instruction_data = ProgramData {
            code: 1,   // balance
            amount: 0, // 0 sol
        };

        let instruction = Instruction::new_with_borsh(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(receiver, false),
                AccountMeta::new(solana_program::system_program::ID, false),
            ],
        );
        println!("instruction: {:#?}", instruction);

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[&payer],
            client.get_latest_blockhash().unwrap(),
        );
        println!("transaction: {:#?}", transaction);

        let tx = client.send_and_confirm_transaction(&transaction);
        println!("tx: {:#?}", tx);

        assert!(tx.is_ok())
    }

    #[test]
    fn test_deposit() {
        let program_id = Pubkey::from_str(PROGRAM_ID.as_str()).unwrap();

        let client =
            RpcClient::new_with_commitment(RPC_URL.as_str(), CommitmentConfig::confirmed());

        let payer = Keypair::from_base58_string(KEYPAIR.as_str());
        println!("payer: {}", payer.pubkey());

        let (receiver, bump_seed) = solana_program::pubkey::Pubkey::find_program_address(
            &[payer.pubkey().as_ref()],
            &program_id,
        );
        println!("receiver: {}", receiver);

        let instruction_data = ProgramData {
            code: 2,         // deposit
            amount: 100_000, // 0.0001 sol
        };

        let instruction = Instruction::new_with_borsh(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(receiver, false),
                AccountMeta::new(solana_program::system_program::ID, false),
            ],
        );
        println!("instruction: {:#?}", instruction);

        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        println!("transaction: {:#?}", transaction);

        transaction.sign(&[&payer], client.get_latest_blockhash().unwrap());

        let tx = client.send_and_confirm_transaction(&transaction);
        println!("tx: {:#?}", tx);

        assert!(tx.is_ok())
    }

    #[test]
    fn test_withdraw() {
        let program_id = Pubkey::from_str(PROGRAM_ID.as_str()).unwrap();

        let client =
            RpcClient::new_with_commitment(RPC_URL.as_str(), CommitmentConfig::confirmed());

        let payer = Keypair::from_base58_string(KEYPAIR.as_str());
        println!("payer: {}", payer.pubkey());

        let (receiver, bump_seed) = solana_program::pubkey::Pubkey::find_program_address(
            &[payer.pubkey().as_ref()],
            &program_id,
        );
        println!("receiver: {}", receiver);

        let instruction_data = ProgramData {
            code: 3,         // withdraw
            amount: 100_000, // 0.0001 sol
        };

        let instruction = Instruction::new_with_borsh(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(receiver, false),
                AccountMeta::new(solana_program::system_program::ID, false),
            ],
        );
        println!("instruction: {:#?}", instruction);

        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        println!("transaction: {:#?}", transaction);

        transaction.sign(&[&payer], client.get_latest_blockhash().unwrap());

        let tx = client.send_and_confirm_transaction(&transaction);
        println!("tx: {:#?}", tx);

        assert!(tx.is_ok())
    }
}
