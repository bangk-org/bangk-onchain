// File: bangk/src/invest/investment.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/invest/investment.rs
// Project: bangk-onchain
// Creation date: Monday 04 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:45:38
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use crate::{
    check_investment, check_project_status,
    instruction::{InvestmentClientArgs, InvestmentClientWithExchangeArgs},
    invest::create_client_investment,
    state::{
        get_mint_metadata,
        mints::BangkMint,
        projects::{check_project_ata, Project, ProjectStatus},
        stable::StableMint,
    },
    utils::tokens::{self, transfer, transfer_with_exchange, ExchangeTrueValue, FreezeCheck},
};
use bangk_onchain_common::{
    check_ata_exists, check_pda_owner, check_spl_program, check_system_program, debug, Error,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

/// Creates a new investment for a client.
///
/// # Parameters
/// * `program_id` - ID of Bangk's program.
/// * `accounts` - Accounts used in the transaction.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 󰴒 Transaction payer,
/// 2. Mint of the stable currency associated to the project,
/// 3. 󰴒 ATA of the stable currency for the client,
/// 4. 󰴒 ATA of the stable currency for the project,
/// 5. 󰴒 Mint of the project's tokens,
/// 6. 󰴒 Dividends Tracker for the project,
/// 6. 󰴒 ATA of the project's tokens for the client,
/// 7. 󰴒 PDA of the client's investment record for the project,
/// 8. System program,
/// 9. SPL 2022 Token program,
/// 10. ATA program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: InvestmentClientArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_stable = next_account_info(accounts_iter)?;
    let stable_client = next_account_info(accounts_iter)?;
    let stable_project = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let tracker_project = next_account_info(accounts_iter)?;
    let token_client = next_account_info(accounts_iter)?;
    let record_client = next_account_info(accounts_iter)?;
    let program_system = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: creating or updating client's investment");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, record_client, tracker_project);
    // check_bangk_owner!(mint_stable, mint_project);
    check_ata_exists!(token_client, stable_client, stable_project);
    // if record_client.lamports() > 0 {
    //     check_same_owner!(stable_client, token_client, record_client);
    // } else {
    //     check_same_owner!(stable_client, token_client);
    // }
    check_investment!(mint_project, record_client, stable_client);
    check_project_status!(mint_project, ProjectStatus::Open);
    check_system_program!(program_system);
    check_spl_program!(program_spl2022);

    let project: Project = get_mint_metadata(mint_project)?.try_into()?;
    check_project_ata(&project, stable_project)?;

    if project.status != ProjectStatus::Open {
        msg!("The project's status does not allow new investements.");
        return Err(Error::InvalidProjectStatus.into());
    }

    if record_client.lamports() == 0 {
        create_client_investment(
            payer,
            mint_project,
            stable_client,
            tracker_project,
            record_client,
            token_client,
            args.record_bump,
        )?;
    }

    let decimals_stable = TryInto::<BangkMint<StableMint>>::try_into(mint_stable.clone())?
        .state()?
        .decimals;
    let cost = args
        .amount
        .checked_mul(project.token_value.into())
        .ok_or(Error::IntegerOverflow)?
        .checked_mul(10_u64.pow(u32::from(decimals_stable)))
        .ok_or(Error::IntegerOverflow)?;

    debug!("Paying the project, (cost = {})", cost);
    transfer(
        stable_client,
        stable_project,
        mint_stable,
        payer,
        program_spl2022,
        FreezeCheck::Neither,
        cost,
    )?;

    debug!("Minting the project's tokens");
    tokens::mint(
        token_client,
        mint_project,
        payer,
        program_spl2022,
        args.amount,
    )
}

/// Creates a new investment for a client.
///
/// # Parameters
/// * `program_id` - ID of Bangk's program.
/// * `accounts` - Accounts used in the transaction.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 󰴒 Transaction payer,
/// 2. Mint of the source currency (the client's),
/// 3. 󰴒 ATA of the stable currency for the client,
/// 4. 󰴒 Bangk's ATA for the source currency,
/// 5. Mint of the target currency (the project's),
/// 6. 󰴒 ATA of the stable currency for the project,
/// 7. 󰴒 Bangk's ATA for the target currency,
/// 8. 󰴒 Mint of the project's tokens,
/// 9. 󰴒 Dividends Tracker for the project,
/// 10. 󰴒 ATA of the project's tokens for the client,
/// 11. 󰴒 PDA of the client's investment record for the project,
/// 12. System program,
/// 13. SPL 2022 Token program,
/// 14. ATA program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process_with_exchange(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: InvestmentClientWithExchangeArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_stable_client = next_account_info(accounts_iter)?;
    let stable_client = next_account_info(accounts_iter)?;
    let exchange_source = next_account_info(accounts_iter)?;
    let mint_stable_project = next_account_info(accounts_iter)?;
    let stable_project = next_account_info(accounts_iter)?;
    let exchange_target = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let tracker_project = next_account_info(accounts_iter)?;
    let token_client = next_account_info(accounts_iter)?;
    let record_client = next_account_info(accounts_iter)?;
    let program_system = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: creating or updating client's investment with exchange");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, record_client, tracker_project);
    // check_bangk_owner!(program_id,
    //     mint_stable_project,
    //     mint_stable_client,
    //     mint_project
    //     exchange_source,
    //     exchange_target
    // );
    check_ata_exists!(token_client, stable_client, stable_project);
    // if record_client.lamports() > 0 {
    //     check_same_owner!(stable_client, token_client, record_client);
    // } else {
    //     check_same_owner!(stable_client, token_client);
    // }
    check_investment!(mint_project, record_client, stable_client);
    check_project_status!(mint_project, ProjectStatus::Open);
    check_system_program!(program_system);
    check_spl_program!(program_spl2022);

    if mint_stable_client.key == mint_stable_project.key {
        return Err(Error::UnecessaryExchange.into());
    }

    let project: Project = get_mint_metadata(mint_project)?.try_into()?;

    let decimals_project = TryInto::<BangkMint<StableMint>>::try_into(mint_stable_project.clone())?
        .state()?
        .decimals;
    let cost = args
        .amount
        .checked_mul(project.token_value.into()) // cost in currency
        .ok_or(Error::IntegerOverflow)?
        .checked_mul(10_u64.pow(u32::from(decimals_project))) // get the number of tokens
        .ok_or(Error::IntegerOverflow)?;

    if record_client.lamports() == 0 {
        create_client_investment(
            payer,
            mint_project,
            stable_client,
            tracker_project,
            record_client,
            token_client,
            args.record_bump,
        )?;
    }

    transfer_with_exchange(
        stable_client,
        stable_project,
        mint_stable_client,
        mint_stable_project,
        exchange_source,
        exchange_target,
        payer,
        cost,
        args.exchange_rate,
        ExchangeTrueValue::Target,
    )?;

    debug!("Minting the project's tokens");
    tokens::mint(
        token_client,
        mint_project,
        payer,
        program_spl2022,
        args.amount,
    )
}
