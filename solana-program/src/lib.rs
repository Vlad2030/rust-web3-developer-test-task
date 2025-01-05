use solana_program::sysvar::Sysvar;

solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &solana_program::pubkey::Pubkey,
    accounts: &[solana_program::account_info::AccountInfo],
    instruction_data: &[u8],
) -> solana_program::entrypoint::ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user_account = solana_program::account_info::next_account_info(accounts_iter)?;
    solana_program::msg!(
        "User account: {}, is signer: {}, is writable: {}",
        user_account.key,
        user_account.is_signer,
        user_account.is_writable,
    );

    let user_pda_account = solana_program::account_info::next_account_info(accounts_iter)?;
    solana_program::msg!(
        "User PDA: {}, is signer: {}, is writable: {}",
        user_pda_account.key,
        user_pda_account.is_signer,
        user_pda_account.is_writable,
    );

    if !user_account.is_signer {
        return Err(solana_program::program_error::ProgramError::MissingRequiredSignature);
    }

    if instruction_data.len() < 9 {
        return Err(solana_program::program_error::ProgramError::InvalidInstructionData);
    }

    solana_program::msg!("Finding PDA");

    let (pda_account, bump_seed) = solana_program::pubkey::Pubkey::find_program_address(
        &[user_account.key.as_ref()],
        program_id,
    );

    solana_program::msg!("Found PDA: {}, seed: {}", pda_account, bump_seed);

    solana_program::msg!("Checking PDA validity");

    if user_pda_account.key != &pda_account {
        return Err(solana_program::program_error::ProgramError::InvalidAccountData);
    }

    solana_program::msg!("Checking PDA data");

    if user_pda_account.data_is_empty() {
        solana_program::msg!("PDA data empty");

        let rent = solana_program::rent::Rent::get()?;
        let rent_required_lamports = rent.minimum_balance(8);

        if **user_account.try_borrow_lamports()? <= rent_required_lamports {
            return Err(solana_program::program_error::ProgramError::InsufficientFunds);
        }

        solana_program::program::invoke_signed(
            &solana_program::system_instruction::create_account(
                user_account.key,
                user_pda_account.key,
                rent_required_lamports,
                8,
                program_id,
            ),
            &[user_account.clone(), user_pda_account.clone()],
            &[&[user_account.key.as_ref(), &[bump_seed]]],
        )?;
    }

    let instruction = InstructionType::unpack(instruction_data)?;

    match instruction {
        InstructionType::Balance => {
            solana_program::msg!("Fetching balance");

            let current_balance = read_balance(user_pda_account)?;

            solana_program::msg!("Current balance: {} lamports", current_balance);
        }

        InstructionType::Deposit => {
            solana_program::msg!("Processing deposit");

            let deposit_amount = u64_from_data(&instruction_data[1..9])?;

            solana_program::msg!("Lamports to deposit: {}", deposit_amount);

            if deposit_amount <= u64::MIN {
                return Err(solana_program::program_error::ProgramError::InsufficientFunds);
            }

            solana_program::program::invoke(
                &solana_program::system_instruction::transfer(
                    user_account.key,
                    user_pda_account.key,
                    deposit_amount,
                ),
                &[user_account.clone(), user_pda_account.clone()],
            )?;

            update_balance(user_pda_account, deposit_amount, true)?;

            solana_program::msg!(
                "Deposit successful. New PDA balance: {}, user account balance updated.",
                read_balance(user_pda_account)?,
            );
        }

        InstructionType::Withdraw => {
            solana_program::msg!("Processing withdrawal");

            let withdraw_amount = u64_from_data(&instruction_data[1..9])?;

            solana_program::msg!("Lamports to withdraw: {}", withdraw_amount);

            let current_balance = read_balance(user_pda_account)?;

            if current_balance < withdraw_amount {
                return Err(solana_program::program_error::ProgramError::InsufficientFunds);
            }

            **user_pda_account.try_borrow_mut_lamports()? -= withdraw_amount;
            **user_account.try_borrow_mut_lamports()? += withdraw_amount;

            update_balance(user_pda_account, withdraw_amount, false)?;

            solana_program::msg!(
                "Withdrawal successful. New PDA balance: {}, user account balance updated.",
                read_balance(user_pda_account)?,
            );
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum InstructionType {
    Balance = 1,
    Deposit = 2,
    Withdraw = 3,
}

impl InstructionType {
    pub fn unpack(input: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        match input[0] {
            1 => Ok(Self::Balance),
            2 => Ok(Self::Deposit),
            3 => Ok(Self::Withdraw),
            _ => Err(solana_program::program_error::ProgramError::InvalidInstructionData),
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Balance => 1,
            Self::Deposit => 2,
            Self::Withdraw => 3,
        }
    }
}

fn read_balance(
    account: &solana_program::account_info::AccountInfo,
) -> Result<u64, solana_program::program_error::ProgramError> {
    let data = account.try_borrow_data()?;
    let balance = u64_from_data(&data)?;

    Ok(balance)
}

fn update_balance(
    account: &solana_program::account_info::AccountInfo,
    amount: u64,
    is_deposit: bool,
) -> solana_program::entrypoint::ProgramResult {
    let mut data = account.try_borrow_mut_data()?;

    let current_balance = u64_from_data(&data)?;

    let new_balance = match is_deposit {
        true => current_balance
            .checked_add(amount)
            .ok_or(solana_program::program_error::ProgramError::InvalidAccountData)?,
        false => current_balance
            .checked_sub(amount)
            .ok_or(solana_program::program_error::ProgramError::InsufficientFunds)?,
    };

    data.copy_from_slice(&new_balance.to_le_bytes());

    Ok(())
}

fn u64_from_data(data: &[u8]) -> Result<u64, solana_program::program_error::ProgramError> {
    if data.len() < 8 {
        return Err(solana_program::program_error::ProgramError::AccountDataTooSmall);
    }

    let parsed = data[..8]
        .try_into()
        .map_err(|_| solana_program::program_error::ProgramError::InvalidAccountData)?;

    Ok(u64::from_le_bytes(parsed))
}
