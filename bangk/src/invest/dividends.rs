// File: bangk/src/invest/dividends.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/invest/dividends.rs
// Project: bangk-onchain
// Creation date: Friday 15 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
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
use spl_token_2022::{extension::StateWithExtensions, state};

use crate::{
    check_investment, check_project_status,
    instruction::{PayInvestmentDividendsArgs, PayInvestmentDividendsWithExchangeArgs},
    state::{
        clients::Investment,
        dividends_tracker::DividendsTracker,
        get_mint_metadata,
        mints::BangkMint,
        pda::{from_account, BangkPda as _},
        projects::{check_project_ata, Project, ProjectStatus},
        stable::StableMint,
    },
    utils::tokens::{transfer, transfer_with_exchange, ExchangeTrueValue, FreezeCheck},
};

/// Pays interests to a batch of clients.
///
/// If this is the last batch (and everything went fine),
/// then the project is updated to set the next payment date.
///
/// # Parameters
/// * `program_id` - The current program's ID.
/// * `accounts` - Accounts used in the transaction.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. Mint of the project's stable currency,
/// 3. 󰴒 ATA of the project's stable coin (from which the payment is taken),
/// 5. 󰴒 ATA of the client's stable coin (must match the project's, otherwise see [`process_with_exchange`]),
/// 4. 󰴒 Mint of the project's tokens,
/// 6. 󰴒 ATA of the client's projects tokens,
/// 7. 󰴒 PDA of the client's [`Investment`] record.
/// 8. SPL 2022 Token Program Account.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: PayInvestmentDividendsArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_stable = next_account_info(accounts_iter)?;
    let ata_stable_project = next_account_info(accounts_iter)?;
    let ata_stable_client = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let tracker_project = next_account_info(accounts_iter)?;
    let ata_token_client = next_account_info(accounts_iter)?;
    let pda_record = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: paying project dividends");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, pda_record);
    // check_bangk_owner!(program_id, mint_stable, mint_project);
    check_ata_exists!(ata_token_client, ata_stable_client, ata_stable_project);
    // check_same_owner!(ata_stable_client, ata_token_client, pda_record);
    check_investment!(mint_project, pda_record, ata_stable_client);
    check_project_status!(mint_project, ProjectStatus::Live);
    check_spl_program!(program_spl2022);

    let decimals_mint_stable = TryInto::<BangkMint<StableMint>>::try_into(mint_stable.clone())?
        .state()?
        .decimals;

    let dividends = check_and_update(
        payer,
        ata_token_client,
        pda_record,
        mint_project,
        tracker_project,
        ata_stable_project,
        args.interest,
        decimals_mint_stable,
    )?;

    transfer(
        ata_stable_project,
        ata_stable_client,
        mint_stable,
        payer,
        program_spl2022,
        FreezeCheck::Neither,
        dividends,
    )
}

/// Pay interests to a client in his preferred currency.
///
/// # Parameters
/// * `program_id` - The current program's ID.
/// * `accounts` - Accounts used in the transaction.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. Mint of the source currency (the project's),
/// 3. 󰴒 ATA of the project's stable coin (from which the payment is taken),
/// 8. 󰴒 ATA of the client's stable coin (must be different from the project's otherwise see [process]),
/// 4. 󰴒 Bangk's ATA for the source currency,
/// 5. Mint of the target currency (the clients'),
/// 6. 󰴒 Bangk's ATA for the target currency,
/// 7. 󰴒 Mint of the project's tokens,
/// 9. 󰴒 ATA of the client's projects tokens,
/// 10. 󰴒 PDA of the client's [`Investment`] record.
/// 11. SPL 2022 Token Program,
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process_with_exchange(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: PayInvestmentDividendsWithExchangeArgs,
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
    let ata_token_client = next_account_info(accounts_iter)?;
    let pda_record = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: paying project dividends with exchange");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, pda_record,);
    // check_bangk_owner!(
    //     program_id,
    //     exchange_source,
    //     exchange_target,
    //     mint_source,
    //     mint_target,
    //     mint_project
    // );
    check_ata_exists!(ata_token_client, ata_stable_client, ata_stable_project);
    // check_same_owner!(ata_stable_client, ata_token_client, pda_record);
    check_investment!(mint_project, pda_record, ata_stable_client);
    check_project_status!(mint_project, ProjectStatus::Live);
    check_spl_program!(program_spl2022);

    if mint_source.key == mint_target.key {
        return Err(Error::UnecessaryExchange.into());
    }

    let decimals_mint_source = TryInto::<BangkMint<StableMint>>::try_into(mint_source.clone())?
        .state()?
        .decimals;

    let dividends = check_and_update(
        payer,
        ata_token_client,
        pda_record,
        mint_project,
        tracker_project,
        ata_stable_project,
        args.interest,
        decimals_mint_source,
    )?;

    transfer_with_exchange(
        ata_stable_project,
        ata_stable_client,
        mint_source,
        mint_target,
        exchange_source,
        exchange_target,
        payer,
        dividends,
        args.exchange_rate,
        ExchangeTrueValue::Source,
    )
}

/// Check the transaction's validity and updates the client's record.
///
/// Performs various checks to make sure the parameters are valid,
/// then updates the client' record and computes how much he should
/// be paid.
///
/// # Parameters
/// * `token_account` - Client's project token account,
/// * `record` - Client's record for the project,
/// * `payer` - Wallet of the transaction fee payer (and signer),
/// * `mint_project` - Mint of project,
/// * `stable_project` - Stable currency ATA for the project,
/// * `rate` - Interest rate,
///
/// # Return
/// * Array of the clients' stable ATAs
/// * Values of the dividends to be paid to each client.
#[allow(clippy::too_many_arguments)]
fn check_and_update<'a>(
    payer: &AccountInfo<'a>,
    token: &AccountInfo<'a>,
    record: &AccountInfo<'a>,
    mint_project: &AccountInfo<'a>,
    tracker_project: &AccountInfo<'a>,
    stable_project: &AccountInfo<'a>,
    rate: u32,
    stable_decimals: u8,
) -> Result<u64, ProgramError> {
    if rate == 0_u32 {
        return Err(Error::NegativeOrNullInterestRate.into());
    }

    // Check that the project's next payment is pending
    let project: Project = get_mint_metadata(mint_project)?.try_into()?;
    let mut tracker: DividendsTracker = from_account(tracker_project)?;
    check_project_ata(&project, stable_project)?;
    if project.status != ProjectStatus::Live {
        return Err(Error::InvalidProjectStatus.into());
    }
    if project.next_payment > get_timestamp()? {
        return Err(Error::DividendPaymentsTriggeredTooSoon.into());
    }
    let token_value = project
        .token_value
        .checked_mul(10_u32.pow(u32::from(stable_decimals)))
        .ok_or(Error::IntegerOverflow)?;

    if tracker.payment_date < project.next_payment && tracker.paid_clients == 0 {
        debug!(
            "Setting payment date on the tracker: {}",
            project.next_payment
        );
        tracker.payment_date = project.next_payment;
    }

    let mut investment: Investment = from_account(record)?;
    // If dividends have already been paid for this round, skip this client.
    if investment.last_payment == tracker.payment_date {
        msg!("client has already been paid for this round");
        return Err(Error::DividendPaymentsTriggeredTooSoon.into());
    }

    // Update the record (can be done now since it's rollbacked in case of future error).
    investment.last_payment = tracker.payment_date;
    investment.save()?;

    // Get the amount invested
    let amount = StateWithExtensions::<state::Account>::unpack(&token.try_borrow_data()?)?
        .base
        .amount;

    let rate = (f64::from(rate) / 1e6_f64) / 100_f64;
    let dividends = (amount
        .checked_mul(token_value.into())
        .ok_or(Error::IntegerOverflow)? as f64
        * rate) as u64;

    tracker.paid_clients = tracker
        .paid_clients
        .checked_add(1)
        .ok_or(Error::IntegerOverflow)?;

    debug!("Tracker:{:#?}", tracker);
    if tracker.paid_clients == tracker.total_clients {
        debug!("round of payments done: updating tracker & project");
        let mut project = project;
        project.update_payment_dates(get_timestamp()?)?;
        tracker.paid_clients = 0;

        let project = BangkMint::new(mint_project, project);
        project.update(payer)?;
    }

    tracker.save()?;

    debug!("Going to payment");

    Ok(dividends)
}
