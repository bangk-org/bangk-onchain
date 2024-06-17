// File: bangk-onchain-common/src/query.rs
// Project: bangk-onchain
// Creation date: Wednesday 24 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 24 July 2024 @ 21:43:46
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use solana_program::{account_info::AccountInfo, pubkey::Pubkey};
use spl_token_2022::{extension::StateWithExtensions, state::Account};

use crate::{Error, Result};

/// Get the owner of a given ATA
///
/// # Parameters
/// * `ata` - ATA for which to retrieve the owner
///
/// # Errors
/// If the given account is not an (existing) ATA
pub fn get_ata_owner(ata: &AccountInfo) -> Result<Pubkey> {
    let state = StateWithExtensions::<Account>::unpack(
        *ata.try_borrow_data()
            .map_err(|_err| Error::InvalidAtaData)?,
    )
    .map_err(|_err| Error::InvalidAtaData)?
    .base;
    Ok(state.owner)
}

/// Get the current timestamp (or a close estimation to it)
///
/// # Errors
// If the clock could not be obtained
#[cfg(not(feature = "no-entrypoint"))]
pub fn get_timestamp() -> Result<i64> {
    use solana_program::{clock::Clock, sysvar::Sysvar as _};
    let clock = Clock::get().map_err(|_err| Error::Clock)?;
    Ok(clock.unix_timestamp)
}

/// Get the current timestamp (or a close estimation to it)
///
/// This is only for tests ! means other programs calling this one won't work
/// where time is concerned, but it's only in projects so we don't really care
/// since that's something we will keep full control off. !
///
/// # Errors
/// If the clock could not be obtained
#[cfg(feature = "no-entrypoint")]
pub fn get_timestamp() -> Result<i64> {
    Ok(chrono::Utc::now().timestamp())
}
