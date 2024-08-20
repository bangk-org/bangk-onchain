// File: bangk/src/invest/transfer.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/invest/transfer.rs
// Project: bangk-onchain
// Creation date: Thursday 27 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:46:37
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/invest/transfer.rs

// Project: bangk-onchain
// Creation date: Tuesday 12 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 02 April 2024 @ 15:37:34
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use crate::{
    check_investment, check_project_status,
    instruction::{TransferInvestmentArgs, TransferInvestmentWithExchangeArgs},
    invest::create_client_investment,
    state::{
        clients::Investment,
        dividends_tracker::DividendsTracker,
        pda::{from_account, BangkPda as _},
        projects::ProjectStatus,
    },
    utils::{
        accounts::close_account,
        tokens::{
            freeze, get_token_amount, thaw, transfer, transfer_with_exchange, ExchangeTrueValue,
            FreezeCheck,
        },
    },
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

/// Transfers investments from one account to another.
///
/// If the destination account does not exist yet, the Associated Token Program and the
/// record is created before the coins are transferred. If the seller sells his last token
/// the record and the ATA are deleted.
///
/// # Parameters
/// * `program_id` - ID of Bangk's program.
/// * `accounts` - Accounts used in the transaction.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. Mint of the stable currency,
/// 3. 󰴒 ATA of the buyer's stable coins,
/// 4. 󰴒 ATA of the seller's stable coins,
/// 5. Mint of the project's tokens,
/// 6. 󰴒 ATA of the buyer's project tokens (created if necessary),
/// 7. 󰴒 ATA of the seller's project tokens,
/// 8. 󰴒 PDA of the buyer's investment record (created if necessary),
/// 9. 󰴒 PDA of the seller's investment record,
/// 10. System program,
/// 11. SPL 2022 Token Program,
/// 12. ATA Program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: TransferInvestmentArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_stable = next_account_info(accounts_iter)?;
    let stable_buyer = next_account_info(accounts_iter)?;
    let stable_seller = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let tracker_project = next_account_info(accounts_iter)?;
    let token_buyer = next_account_info(accounts_iter)?;
    let token_seller = next_account_info(accounts_iter)?;
    let record_buyer = next_account_info(accounts_iter)?;
    let record_seller = next_account_info(accounts_iter)?;
    let program_system = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: transferring project tokens");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, record_buyer, record_seller, tracker_project);
    // check_bangk_owner!(mint_buyer, mint_seller, mint_project);
    check_ata_exists!(token_seller, token_buyer);
    // if record_buyer.lamports() > 0 {
    //     check_same_owner!(stable_buyer, token_buyer, record_buyer);
    // } else {
    //     check_same_owner!(stable_buyer, token_buyer);
    // }
    // check_same_owner!(stable_seller, token_seller, record_seller);
    check_investment!(mint_project, record_seller, stable_seller);
    check_investment!(mint_project, record_buyer, stable_buyer);
    check_project_status!(mint_project, ProjectStatus::Live);
    check_system_program!(program_system);
    check_spl_program!(program_spl2022);

    let tracker: DividendsTracker = from_account(tracker_project)?;
    if tracker.paid_clients > 0 {
        return Err(Error::PendingPayments.into());
    }

    if record_buyer.lamports() == 0 {
        create_client_investment(
            payer,
            mint_project,
            stable_buyer,
            tracker_project,
            record_buyer,
            token_buyer,
            args.record_bump,
        )?;
    }

    debug!("Transferring payment");
    transfer(
        stable_buyer,
        stable_seller,
        mint_stable,
        payer,
        program_spl2022,
        FreezeCheck::Neither,
        args.cost,
    )?;

    debug!("Transferring tokens");
    thaw(program_spl2022, &[token_seller], mint_project, payer)?;
    transfer(
        token_seller,
        token_buyer,
        mint_project,
        payer,
        program_spl2022,
        FreezeCheck::Payee,
        args.amount,
    )?;

    // If the seller has no tokens left, remove his accounts.
    if get_token_amount(token_seller)? == 0 {
        debug!("No token left in seller account: closing accounts.");
        close_account(payer, token_seller, program_spl2022)?;
        let record: Investment = from_account(record_seller)?;
        record.delete(payer)?;

        let mut tracker = tracker;
        tracker.total_clients = tracker
            .total_clients
            .checked_sub(1)
            .ok_or(Error::IntegerOverflow)?;
        tracker.save()?;
    } else {
        freeze(program_spl2022, &[token_seller], mint_project, payer)?;
    }

    Ok(())
}

/// Transfers investments from one account to another paying in a different currency
/// than the one received.
///
/// If the destination account does not exist yet, the Associated Token Program and the
/// record is created before the coins are transferred. If the seller sells his last token
/// the record and the ATA are deleted.
///
/// # Parameters
/// * `program_id` - ID of Bangk's program,
/// * `accounts` - Accounts used in the transaction,
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. Mint of the source currency (the buyer's),
/// 3. 󰴒 ATA of the buyer's stable coins,
/// 4. 󰴒 ATA of Bangk's source currency,
/// 5. Mint of the target currency (the seller's),
/// 6. 󰴒 ATA of the seller's stable coins,
/// 7. 󰴒 ATA of Bangk's target currency,
/// 8. Mint of the project's tokens,
/// 9. 󰴒 ATA of the buyer's project tokens (created if necessary),
/// 10. 󰴒 ATA of the seller's project tokens,
/// 11. 󰴒 PDA of the buyer's investment record (created if necessary),
/// 12. 󰴒 PDA of the seller's investment record,
/// 13. System program,
/// 14. SPL 2022 Token Program,
/// 15. ATA Program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process_with_exchange(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: TransferInvestmentWithExchangeArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_stable_buyer = next_account_info(accounts_iter)?;
    let stable_buyer = next_account_info(accounts_iter)?;
    let exchange_source = next_account_info(accounts_iter)?;
    let mint_stable_seller = next_account_info(accounts_iter)?;
    let stable_seller = next_account_info(accounts_iter)?;
    let exchange_target = next_account_info(accounts_iter)?;
    let mint_project = next_account_info(accounts_iter)?;
    let tracker_project = next_account_info(accounts_iter)?;
    let token_buyer = next_account_info(accounts_iter)?;
    let token_seller = next_account_info(accounts_iter)?;
    let record_buyer = next_account_info(accounts_iter)?;
    let record_seller = next_account_info(accounts_iter)?;
    let program_system = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: transferring project tokens with exchange");

    // Integrity checks
    // check_signers!(accounts, payer);
    check_pda_owner!(program_id, record_buyer, record_seller, tracker_project);
    // check_bangk_owner!(
    //     mint_stable_buyer,
    //     mint_stable_buyer,
    //     mint_project
    //     exchange_source,
    //     exchange_target
    // );
    check_ata_exists!(token_seller, token_buyer);
    // if record_buyer.lamports() > 0 {
    //     check_same_owner!(stable_buyer, token_buyer, record_buyer);
    // } else {
    //     check_same_owner!(stable_buyer, token_buyer);
    // }
    // check_same_owner!(stable_seller, token_seller, record_seller);
    check_investment!(mint_project, record_seller, stable_seller);
    check_investment!(mint_project, record_buyer, stable_buyer);
    check_project_status!(mint_project, ProjectStatus::Live);
    check_system_program!(program_system);
    check_spl_program!(program_spl2022);

    let tracker: DividendsTracker = from_account(tracker_project)?;
    if tracker.paid_clients > 0 {
        return Err(Error::PendingPayments.into());
    }

    if record_buyer.lamports() == 0 {
        create_client_investment(
            payer,
            mint_project,
            stable_buyer,
            tracker_project,
            record_buyer,
            token_buyer,
            args.record_bump,
        )?;
    }

    debug!("Transferring payment");
    transfer_with_exchange(
        stable_buyer,
        stable_seller,
        mint_stable_buyer,
        mint_stable_seller,
        exchange_source,
        exchange_target,
        payer,
        args.cost,
        args.exchange_rate,
        ExchangeTrueValue::Target,
    )?;

    debug!("Transferring tokens");
    thaw(program_spl2022, &[token_seller], mint_project, payer)?;
    transfer(
        token_seller,
        token_buyer,
        mint_project,
        payer,
        program_spl2022,
        FreezeCheck::Payee,
        args.amount,
    )?;

    // If the seller has no tokens left, remove his accounts.
    if get_token_amount(token_seller)? == 0 {
        let mut tracker = tracker;
        if tracker.paid_clients > 0 {
            return Err(Error::PendingPayments.into());
        }

        debug!("No token left in seller account: closing accounts.");
        close_account(payer, token_seller, program_spl2022)?;
        let lamports = payer.lamports();
        **payer.lamports.borrow_mut() = record_seller
            .lamports()
            .checked_add(lamports)
            .ok_or(Error::RentExemptionRetrieval)?;
        **record_seller.lamports.borrow_mut() = 0;

        tracker.total_clients = tracker
            .total_clients
            .checked_sub(1)
            .ok_or(Error::IntegerOverflow)?;
        tracker.save()?;
    } else {
        freeze(program_spl2022, &[token_seller], mint_project, payer)?;
    }

    Ok(())
}
