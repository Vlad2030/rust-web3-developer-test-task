use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    msg!("Solana Program");

    let accounts_iter = &mut _accounts.iter();
    let user_account = next_account_info(accounts_iter)?;
    let program_account = next_account_info(accounts_iter)?;
    let user_balance_account = next_account_info(accounts_iter)?;

    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // 0 = Balance, 1 = Deposit, 2 = Withdraw
    match _instruction_data[0] {
        0 => {
            msg!("Fetching balance");

            let user_balance_data = user_balance_account.try_borrow_data()?;
            let current_balance =
                u64::from_le_bytes(user_balance_data[..8].try_into().unwrap_or([0; 8]));

            msg!("Current balance: {} lamports", current_balance);
        }
        1 => {
            msg!("Processing deposit");

            let deposit_amount = u64::from_le_bytes(
                _instruction_data[1..9]
                    .try_into()
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            );

            **program_account.try_borrow_mut_lamports()? += deposit_amount;
            **user_account.try_borrow_mut_lamports()? -= deposit_amount;

            let mut user_balance_data = user_balance_account.try_borrow_mut_data()?;
            let current_balance =
                u64::from_le_bytes(user_balance_data[..8].try_into().unwrap_or([0; 8]));
            let new_balance = current_balance + deposit_amount;
            user_balance_data[..8].copy_from_slice(&new_balance.to_le_bytes());

            msg!(
                "Deposited {} lamports. New balance: {} lamports",
                deposit_amount,
                new_balance
            );
        }
        2 => {
            msg!("Processing withdrawal");

            let withdraw_amount = u64::from_le_bytes(
                _instruction_data[1..9]
                    .try_into()
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            );

            if **program_account.lamports.borrow() < withdraw_amount {
                return Err(ProgramError::InsufficientFunds);
            }

            let mut user_balance_data = user_balance_account.try_borrow_mut_data()?;
            let current_balance =
                u64::from_le_bytes(user_balance_data[..8].try_into().unwrap_or([0; 8]));
            if current_balance < withdraw_amount {
                return Err(ProgramError::InsufficientFunds);
            }

            let new_balance = current_balance - withdraw_amount;
            user_balance_data[..8].copy_from_slice(&new_balance.to_le_bytes());

            **program_account.try_borrow_mut_lamports()? -= withdraw_amount;
            **user_account.try_borrow_mut_lamports()? += withdraw_amount;

            msg!(
                "Withdrew {} lamports. Remaining balance: {} lamports",
                withdraw_amount,
                new_balance
            );
        }
        _ => {
            msg!("Invalid instruction");

            return Err(ProgramError::InvalidInstructionData);
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };
    use solana_program_test::{processor, ProgramTest};
    use solana_sdk::{
        account::Account,
        signature::{Keypair, Signer},
        transaction::Transaction,
        commitment_config::CommitmentLevel,
    };
    use std::str::FromStr;

    #[tokio::test]
    async fn test_get_balance() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::from_str("ELEHyLRJYNt8B5iBHzhGPTajuhchhXP5sx5RJAD7bvCS")?;
        // let program_id = Pubkey::new_unique();
        println!("program_id: {:#?}", program_id);

        let user_account = Keypair::new();
        println!("user_account: {:#?}", user_account.pubkey());

        let program_account = Keypair::new();
        println!("program_account: {:#?}", program_account.pubkey());

        let user_balance_account = Keypair::new();
        println!("user_balance_account: {:#?}", user_balance_account.pubkey());

        let mut program_test = ProgramTest::new(
            "solana_program",
            program_id,
            processor!(process_instruction),
        );

        program_test.add_account(
            user_account.pubkey(),
            Account {
                lamports: 1000,
                data: vec![0; 0],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        program_test.add_account(
            program_account.pubkey(),
            Account {
                lamports: 1000000000,
                data: vec![0; 0],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        program_test.add_account(
            user_balance_account.pubkey(),
            Account {
                lamports: 0,
                data: vec![0; 8],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        let deposit_amount = 100;
        println!("deposit_amount: {:#?}", deposit_amount);

        let instruction_data = vec![0];
        println!("instruction_data: {:#?}", instruction_data);

        let instruction = Instruction::new_with_borsh(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(user_account.pubkey(), true),
                AccountMeta::new(program_account.pubkey(), false),
                AccountMeta::new(user_balance_account.pubkey(), false),
            ],
        );
        println!("instruction: {:#?}", instruction);

        let mut transaction =
            Transaction::new_with_payer(&[instruction], Some(&user_account.pubkey()));
        println!("transaction: {:#?}", transaction);

        transaction.sign(&[&user_account], recent_blockhash);

        let result = banks_client.process_transaction(transaction).await;
        println!("result: {:#?}", result);

        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_deposit() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::from_str("ELEHyLRJYNt8B5iBHzhGPTajuhchhXP5sx5RJAD7bvCS")?;
        // let program_id = Pubkey::new_unique();
        println!("program_id: {:#?}", program_id);

        let user_account = Keypair::new();
        println!("user_account: {:#?}", user_account.pubkey());

        let program_account = Keypair::new();
        println!("program_account: {:#?}", program_account.pubkey());

        let user_balance_account = Keypair::new();
        println!("user_balance_account: {:#?}", user_balance_account.pubkey());

        let mut program_test = ProgramTest::new(
            "solana_program",
            program_id,
            processor!(process_instruction),
        );

        program_test.add_account(
            user_account.pubkey(),
            Account {
                lamports: 1000,
                data: vec![0; 0],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        program_test.add_account(
            program_account.pubkey(),
            Account {
                lamports: 1000000000,
                data: vec![0; 0],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        program_test.add_account(
            user_balance_account.pubkey(),
            Account {
                lamports: 0,
                data: vec![0; 8],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        let deposit_amount = 100;
        println!("deposit_amount: {:#?}", deposit_amount);

        let instruction_data = vec![1, deposit_amount as u8, (deposit_amount >> 8) as u8];
        println!("instruction_data: {:#?}", instruction_data);

        let instruction = Instruction::new_with_borsh(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(user_account.pubkey(), true),
                AccountMeta::new(program_account.pubkey(), false),
                AccountMeta::new(user_balance_account.pubkey(), false),
            ],
        );
        println!("instruction: {:#?}", instruction);

        let mut transaction =
            Transaction::new_with_payer(&[instruction], Some(&user_account.pubkey()));
        transaction.sign(&[&user_account], recent_blockhash);
        println!("transaction: {:#?}", transaction);

        let result = banks_client.process_transaction(transaction).await;
        println!("result: {:#?}", result);

        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_sol_program() -> Result<(), Box<dyn std::error::Error>> {
        let program_id = Pubkey::from_str("ELEHyLRJYNt8B5iBHzhGPTajuhchhXP5sx5RJAD7bvCS")?;
        // let program_id = Pubkey::new_unique();
        println!("program_id: {:#?}", program_id);

        let user_account = Keypair::new();
        println!("user_account: {:#?}", user_account.pubkey());

        let program_account = Keypair::new();
        println!("program_account: {:#?}", program_account.pubkey());

        let user_balance_account = Keypair::new();
        println!("user_balance_account: {:#?}", user_balance_account.pubkey());

        let mut program_test = ProgramTest::new(
            "solana_program",
            program_id,
            processor!(process_instruction),
        );

        program_test.add_account(
            user_account.pubkey(),
            Account {
                lamports: 1000,
                data: vec![0; 0],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        program_test.add_account(
            program_account.pubkey(),
            Account {
                lamports: 1000000000,
                data: vec![0; 0],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        program_test.add_account(
            user_balance_account.pubkey(),
            Account {
                lamports: 0,
                data: vec![0; 8],
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        let deposit_amount = 100;
        println!("deposit_amount: {:#?}", deposit_amount);

        let instruction_data = vec![1, deposit_amount as u8, (deposit_amount >> 8) as u8];
        println!("instruction_data: {:#?}", instruction_data);

        let instruction = Instruction::new_with_borsh(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(user_account.pubkey(), true),
                AccountMeta::new(program_account.pubkey(), false),
                AccountMeta::new(user_balance_account.pubkey(), false),
            ],
        );
        println!("instruction: {:#?}", instruction);

        let mut transaction =
            Transaction::new_with_payer(&[instruction], Some(&user_account.pubkey()));
        transaction.sign(&[&user_account], recent_blockhash);
        println!("transaction: {:#?}", transaction);

        let result = banks_client.process_transaction(transaction).await;
        println!("result: {:#?}", result);


        let balance_data = banks_client
            .get_account(user_balance_account.pubkey())
            .await?
            .unwrap()
            .data;
        println!("balance_data: {:#?}", balance_data);

        let current_balance = u64::from_le_bytes(balance_data[..8].try_into().unwrap_or([0; 8]));
        println!("current_balance: {:#?}", current_balance);

        assert_eq!(current_balance, deposit_amount as u64);

        let withdraw_amount = 50;
        println!("withdraw_amount: {:#?}", withdraw_amount);

        let instruction_data = vec![2, withdraw_amount as u8, (withdraw_amount >> 8) as u8];
        println!("instruction_data: {:#?}", instruction_data);

        let instruction = Instruction::new_with_borsh(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(user_account.pubkey(), true),
                AccountMeta::new(program_account.pubkey(), false),
                AccountMeta::new(user_balance_account.pubkey(), false),
            ],
        );
        println!("instruction: {:#?}", instruction);

        let mut transaction =
            Transaction::new_with_payer(&[instruction], Some(&user_account.pubkey()));
        println!("transaction: {:#?}", transaction);

        transaction.sign(&[&user_account], recent_blockhash);

        let result = banks_client.process_transaction(transaction).await;
        println!("result: {:#?}", result);

        let balance_data = banks_client
            .get_account(user_balance_account.pubkey())
            .await?
            .unwrap()
            .data;
        println!("balance_data: {:#?}", balance_data);

        let current_balance = u64::from_le_bytes(balance_data[..8].try_into().unwrap_or([0; 8]));
        println!("current_balance: {:#?}", current_balance);

        assert_eq!(
            current_balance,
            deposit_amount as u64 - withdraw_amount as u64
        );

        Ok(())
    }
}
