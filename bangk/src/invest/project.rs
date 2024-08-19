// File: bangk/src/invest/project.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/invest/project.rs
// Project: bangk-onchain
// Creation date: Thursday 14 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:45:49
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{
    check_ata_exists, check_pda_owner, check_spl_program, debug, get_timestamp, Error,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token_2022::{
    extension::StateWithExtensions,
    state::{self, AccountState},
};

use crate::{
    check_investment,
    instruction::{ChangeProjectStatusArgs, CreateInvestProjecArgs, ExchangeRateArgs},
    state::{
        clients::Investment,
        dividends_tracker::DividendsTracker,
        get_mint_metadata, get_state,
        mints::BangkMint,
        pda::{from_account, BangkPda as _},
        projects::{check_project_ata, Project, ProjectStatus},
        stable::StableMint,
    },
    utils::tokens::{burn, transfer, transfer_with_exchange, ExchangeTrueValue, FreezeCheck},
};

/// Initializes a new Bangk Invest project
///
/// At this step, the PDA for the project's data is created and filled with initialized data,
/// then the token mint associated to the project is created.
///
/// # Parameters
/// * `program_id` - The ID of the current program,
/// * `accounts` - Accounts used in the transaction,
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. The delegate on the mint,
/// 3. 󰴒 Mint of the project (will be created),
/// 4. 󰴒 PDA for the dividends tracker,
/// 5. System Program,
/// 6. SPL 2022 Token Program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn create(accounts: &[AccountInfo], args: CreateInvestProjecArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let delegate = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let dividends_tracker = next_account_info(accounts_iter)?;

    msg!("Bangk: creating new project");

    // check_signers!(accounts, payer);

    let project = args.project;

    if mint_project.lamports() != 0 {
        return Err(Error::ProjectAlreadyInitialized.into());
    }

    debug!("Creating project's mint {}", mint_project.key);
    let name = project.name.clone();

    let mint_bump = project.seed_bump;
    let mint = BangkMint::new(mint_project, project);
    mint.create(payer, delegate, 0, &AccountState::Frozen, mint_bump)?;

    let tracker = DividendsTracker::new(dividends_tracker.clone(), mint_project.key, args.bump);
    tracker.create(payer)?;

    msg!("Project {} was successfully initialized", name);
    Ok(())
}

/// Update a project to set it from Open status to Live status.
///
/// # Parameters
/// * `accounts` - Accounts accessed by the transaction.
/// * `args` - New status for the project.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. 󰴒 Mint of the project,
/// 3. 󰴒 PDA for the dividends tracker,
/// 4. SPL 2022 Token Program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn change_project_status(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ChangeProjectStatusArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let dividends_tracker = next_account_info(accounts_iter)?;

    msg!("Bangk: changing project status");

    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, dividends_tracker);
    // check_bangk_owner!(mint_project);

    let mut project: Project = get_mint_metadata(mint_project)?.try_into()?;

    let supply = get_state(mint_project)?.supply;
    let tracker: DividendsTracker = from_account(dividends_tracker)?;

    match args.status {
        ProjectStatus::Live => {
            if project.status != ProjectStatus::Open {
                return Err(Error::InvalidProjectStatus.into());
            }
            debug!("launching project");
            project.launch(get_timestamp()?)?;
        }
        ProjectStatus::Closed => {
            if project.status != ProjectStatus::Live {
                return Err(Error::InvalidProjectStatus.into());
            }
            if tracker.paid_clients != 0 && tracker.total_clients != 0 {
                return Err(Error::PendingPayments.into());
            }
            if supply != 0 {
                return Err(Error::CannotCloseMintWithSupply.into());
            }
            project.close();
        }
        ProjectStatus::Cancelled => {
            if project.status != ProjectStatus::Open {
                return Err(Error::InvalidProjectStatus.into());
            }
            if tracker.paid_clients != 0 && tracker.total_clients != 0 {
                return Err(Error::PendingPayments.into());
            }
            if supply != 0 {
                return Err(Error::CannotCloseMintWithSupply.into());
            }
            project.cancel();
        }
        ProjectStatus::Open => {
            return Err(Error::InvalidProjectStatus.into());
        }
    }

    debug!("updating project's mint");
    let mint = BangkMint::new(mint_project, project);
    mint.update(payer)?;
    debug!("change done");
    Ok(())
}

/// Update a project to set it from Open status to Cancel status.
///
/// All clients are reimbursed.
///
/// # Parameters
/// * `program_id` - ID of the current program.
/// * `accounts` - Accounts accessed by the function.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 󰴒 Transaction payer,
/// 2. Mint of the project's (and the client's) currency,
/// 4. 󰴒 ATA for the project's stable coins,
/// 3. 󰴒 ATA of the client's stable coin (must match the project's, otherwise see [`reimburse_client_with_exchange`]),
/// 5. 󰴒 Mint of the project,
/// 6. 󰴒 ATA of the client's projects tokens (will be destroyed),
/// 7. 󰴒 PDA of the client's [`Investment`] record (will be destroyed).
/// 8. SPL 2022 Token Program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn reimburse_client(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_stable = next_account_info(accounts_iter)?;
    let ata_stable_project = next_account_info(accounts_iter)?;
    let ata_stable_client = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let tracker_project = next_account_info(accounts_iter)?;
    let ata_project_client = next_account_info(accounts_iter)?;
    let pda_record = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: reimbursing project's investment");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, pda_record);
    // check_bangk_owner!(mint_stable, mint_project);
    check_ata_exists!(ata_stable_project, ata_stable_client, ata_project_client);
    // check_same_owner!(ata_stable_client, ata_project_client, pda_record);
    check_investment!(mint_project, pda_record, ata_stable_client);
    check_spl_program!(program_spl2022);

    let decimals_mint_stable = TryInto::<BangkMint<StableMint>>::try_into(mint_stable.clone())?
        .state()?
        .decimals;

    let amount = check_and_update(
        payer,
        ata_project_client,
        mint_project,
        tracker_project,
        ata_stable_project,
        program_spl2022,
        decimals_mint_stable,
    )?;

    // Reimburse the client
    transfer(
        ata_stable_project,
        ata_stable_client,
        mint_stable,
        payer,
        program_spl2022,
        FreezeCheck::Neither,
        amount,
    )?;

    let lamports = payer.lamports();
    **payer.lamports.borrow_mut() = pda_record
        .lamports()
        .checked_add(lamports)
        .ok_or(Error::RentExemptionRetrieval)?;
    **pda_record.lamports.borrow_mut() = 0;
    Ok(())
}

/// Update a project to set it from Open status to Cancel status.
///
/// All clients are reimbursed with currency exchange.
///
/// # Parameters
/// * `program_id` - ID of the current program.
/// * `accounts` - Accounts accessed by the function.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 3. Mint of the source currency (the project's),
/// 4. 󰴒 ATA for the project's stable coins,
/// 9. 󰴒 ATA of the client's stable coin (must be different from the project's, otherwise see [`reimburse_client`]),
/// 5. 󰴒 Bangk Exchange in source currency (the project's),
/// 6. Mint of the target currency (the client's),
/// 7. 󰴒 Bangk Exchange in target currency (the client's),
/// 8. 󰴒 Mint of the project,
/// 10. 󰴒 ATA of the client's projects tokens (will be destroyed),
/// 11. 󰴒 PDA of the client's [`Investment`] record (will be destroyed).
/// 12. SPL 2022 Token Program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn reimburse_client_with_exchange(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ExchangeRateArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_source = next_account_info(accounts_iter)?;
    let ata_stable_project = next_account_info(accounts_iter)?;
    let ata_stable_client = next_account_info(accounts_iter)?;
    let exchange_source = next_account_info(accounts_iter)?;
    let mint_target = next_account_info(accounts_iter)?;
    let exchange_target = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let tracker_project = next_account_info(accounts_iter)?;
    let ata_project_client = next_account_info(accounts_iter)?;
    let pda_record = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: reimbursing project's investment with exchange");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, pda_record);
    // check_bangk_owner!(
    //     program_id,
    //     exchange_source,
    //     exchange_target
    //     mint_source,
    //     mint_target,
    //     mint_project
    // );
    check_ata_exists!(ata_stable_project, ata_stable_client, ata_project_client);
    // check_same_owner!(ata_stable_client, ata_project_client, pda_record);
    check_investment!(mint_project, pda_record, ata_stable_client);
    check_spl_program!(program_spl2022);

    if mint_source.key == mint_target.key {
        return Err(Error::UnecessaryExchange.into());
    }

    let decimals_mint_source = TryInto::<BangkMint<StableMint>>::try_into(mint_source.clone())?
        .state()?
        .decimals;

    let amount = check_and_update(
        payer,
        ata_project_client,
        mint_project,
        tracker_project,
        ata_stable_project,
        program_spl2022,
        decimals_mint_source,
    )?;

    // Reimburse the client
    transfer_with_exchange(
        ata_stable_project,
        ata_stable_client,
        mint_source,
        mint_target,
        exchange_source,
        exchange_target,
        payer,
        amount,
        args.exchange_rate,
        ExchangeTrueValue::Source,
    )?;

    let record: Investment = from_account(pda_record)?;
    record.delete(payer)?;

    Ok(())
}

/// Checks that the transaction is valid and updates mints & records.
///
/// # Parameters
///
/// `token` - Client's token ATA,
/// * `payer` - Payer & signer account,
/// * `ata_project` - Stable currency's ATA for the project,
/// * `program_spl2022` - SPL 2022 Token Program.
///
/// # Returns
///
/// * Number of coins to pay.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
fn check_and_update<'a>(
    payer: &AccountInfo<'a>,
    token: &AccountInfo<'a>,
    mint_project: &AccountInfo<'a>,
    tracker_project: &AccountInfo<'a>,
    ata_project: &AccountInfo<'a>,
    program_spl2022: &AccountInfo<'a>,
    stable_decimals: u8,
) -> Result<u64, ProgramError> {
    let project: Project = get_mint_metadata(mint_project)?.try_into()?;
    check_project_ata(&project, ata_project)?;
    let token_value = project
        .token_value
        .checked_mul(10_u32.pow(u32::from(stable_decimals)))
        .ok_or(Error::IntegerOverflow)?;

    let mut tracker: DividendsTracker = from_account(tracker_project)?;
    if tracker.paid_clients != 0 && tracker.total_clients != 0 {
        return Err(Error::PendingPayments.into());
    }

    // Get the amount to pay
    let nb_tokens = StateWithExtensions::<state::Account>::unpack(&token.try_borrow_data()?)?
        .base
        .amount;
    let amount = nb_tokens
        .checked_mul(token_value.into())
        .ok_or(Error::IntegerOverflow)?;

    // Burn and close the token account
    burn(token, mint_project, payer, program_spl2022, nb_tokens, true)?;

    // Record the client's suppression in the tracker
    tracker.total_clients = tracker
        .total_clients
        .checked_add(1)
        .ok_or(Error::IntegerOverflow)?;
    tracker.save()?;

    Ok(amount)
}
