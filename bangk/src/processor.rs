// File: bangk/src/processor.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/processor.rs
// Project: bangk-onchain
// Creation date: Wednesday 06 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:46:37
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{
    check_ata_program, check_spl_program, check_system_program, debug, Error,
};
use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey,
    pubkey::Pubkey,
};
use spl_token_2022::state::AccountState;

use crate::{
    instruction::{BangkInstruction, CreateStableCoinArgs},
    invest::{self},
    stable::{self},
    state::{mint_data::MintData as _, mints::BangkMint, stable::StableMint},
    utils::accounts::create_ata,
};

include!(concat!(env!("OUT_DIR"), "/keys.rs"));

/// Main processor for the program
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let Ok(payload) = BangkInstruction::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };
    match payload {
        // Administration instructions
        // General instructions
        BangkInstruction::CreateClientAccount => create_client_ata(accounts),
        // Stable coins
        BangkInstruction::CreateStableCoin(args) => initialize(accounts, args),
        BangkInstruction::MintStableCoin(args) => stable::mint::process(accounts, args),
        BangkInstruction::TransferStableCoin(args) => {
            stable::transfer::process(program_id, accounts, args)
        }
        BangkInstruction::ExchangeStableCoin(args) => {
            stable::exchange::process(program_id, accounts, args)
        }
        BangkInstruction::BurnStableCoin(args) => stable::burn::process(accounts, args),

        // Invest project
        BangkInstruction::CreateInvestProject(args) => invest::project::create(accounts, args),
        BangkInstruction::InvestmentClient(args) => {
            invest::investment::process(program_id, accounts, args)
        }
        BangkInstruction::InvestmentClientWithExchange(args) => {
            invest::investment::process_with_exchange(program_id, accounts, args)
        }
        BangkInstruction::TransferInvestment(args) => {
            invest::transfer::process(program_id, accounts, args)
        }
        BangkInstruction::TransferInvestmentWithExchange(args) => {
            invest::transfer::process_with_exchange(program_id, accounts, args)
        }
        BangkInstruction::PayInvestmentDividends(args) => {
            invest::dividends::process(program_id, accounts, args)
        }
        BangkInstruction::PayInvestmentDividendsWithExchange(args) => {
            invest::dividends::process_with_exchange(program_id, accounts, args)
        }
        BangkInstruction::ChangeProjectStatus(args) => {
            invest::project::change_project_status(program_id, accounts, args)
        }
        BangkInstruction::ReimburseInvestProject => {
            invest::project::reimburse_client(program_id, accounts)
        }
        BangkInstruction::ReimburseInvestProjectWithExchange(args) => {
            invest::project::reimburse_client_with_exchange(program_id, accounts, args)
        }
    }
}

/// Initializes a stable coin mint.
///
/// # Parameters
///
/// * `program_id` - ID of the current program,
/// * `accounts` - Accounts used by the transaction,
/// * `args` - Arguments to the instruction.
///
/// # Accounts
///
/// 1. 󰴹 Transaction payer,
/// 2. Mint delegate,
/// 3. 󰴒 Mint of the stable coin to create.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
fn initialize(accounts: &[AccountInfo], args: CreateStableCoinArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let bangk = next_account_info(accounts_iter)?;
    let delegate = next_account_info(accounts_iter)?;
    let pda_mint = next_account_info(accounts_iter)?;

    // check_signers!(accounts, bangk);

    let stable_mint = StableMint {
        name: args.currency.clone(),
        symbol: args.symbol.clone(),
        uri: args.uri,
    };

    let (address, bump) = StableMint::get_address(&args.symbol);

    if address != *pda_mint.key {
        msg!(
            "The address given ({}) does not match the computed one ({}) for currency {}",
            pda_mint.key,
            address,
            args.currency
        );
        return Err(ProgramError::InvalidAccountData);
    }

    if pda_mint.lamports() != 0 {
        msg!("{} Mint has already been initialized.", args.currency);
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    debug!(
        "Creating {} Mint at address {}",
        args.currency, pda_mint.key
    );
    let mint = BangkMint::new(pda_mint, stable_mint);
    mint.create(
        bangk,
        delegate,
        args.decimals,
        &AccountState::Initialized,
        bump,
    )?;
    msg!("{} Coin is now open for business!", args.currency);

    Ok(())
}

/// Create a client's ATA (for project investments or stable coins).
///
/// # Parameters
/// * `accounts` - Accounts used in the transaction,
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. 󰴹 Owner of the account in which the tokens will be minted,
/// 3. Stable mint for which the ATA will be create,
/// 4. 󰴒 ATA to be created,
/// 5. System program,
/// 6. SPL 2022 Token program,
/// 7. Associated Token Account program.
///
/// # Errors
/// If the wrong number of accounts was given, etc.
pub fn create_client_ata(accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let client = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let ata = next_account_info(accounts_iter)?;
    let program_system = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;
    let program_ata = next_account_info(accounts_iter)?;

    debug!("Creating ATA {} of client {}.", ata.key, client.key);
    // Integrity checks
    // check_signers!(accounts, payer);
    // check_bangk_owner!(mint);
    check_system_program!(program_system);
    check_spl_program!(program_spl2022);
    check_ata_program!(program_ata);

    if ata.lamports() != 0 {
        return Err(Error::AccountAlreadyExists.into());
    }
    create_ata(ata, client, mint, payer, program_spl2022, program_system)
}
