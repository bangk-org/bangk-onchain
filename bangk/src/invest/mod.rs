// File: bangk/src/invest/mod.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/invest/mod.rs
// Project: bangk-onchain
// Creation date: Thursday 23 November 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:45:45
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! The Solana On-Chain program's module for Bangk's Invest Token.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use bangk_onchain_common::{debug, get_ata_owner, Error};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult};

use crate::state::{
    clients::Investment,
    dividends_tracker::DividendsTracker,
    pda::{from_account, BangkPda as _},
};

/// Handles dividend payments
pub mod dividends;
/// Handles a client's investment.
pub mod investment;
/// Performs all operations related to a project's creation or update
pub mod project;
/// Handles the transfer of investments from one client to another.
pub mod transfer;

/// Create both an investment record and an ATA for a client's investment.
///
/// # Parameters
/// * `accounts` - Accounts used to create the investment
/// * `record_bump` - Seed bump used to derive the record's PDA.
///
/// # Accounts
/// 1. 󰴹 󰴒 Transaction payer,
/// 2. Mint of the project,
/// 3. Stable coin currency used by the client,
/// 4. Wallet of the client,
/// 5. 󰴒 PDA of the investment record,
/// 6. 󰴒 ATA of the client for the project,
/// 7. System program,
/// 8. SPL 2022 Token Program.
///
/// # Errors
/// If the investment could not be created.
pub fn create_client_investment<'a>(
    payer: &AccountInfo<'a>,
    project: &AccountInfo<'a>,
    ata_stable: &AccountInfo<'a>,
    tracker: &AccountInfo<'a>,
    record: &AccountInfo<'a>,
    token: &AccountInfo<'a>,
    bump: u8,
) -> ProgramResult {
    debug!("Creating client record {}", record.key,);
    let client = get_ata_owner(token)?;

    let investment = Investment::new(record.clone(), &client, project.key, ata_stable.key, bump);
    investment.create(payer)?;

    let mut tracker: DividendsTracker = from_account(tracker)?;
    tracker.total_clients = tracker
        .total_clients
        .checked_add(1)
        .ok_or(Error::IntegerOverflow)?;
    tracker.save()?;

    Ok(())
}
