// File: bangk-onchain-common/src/lib.rs
// Project: bangk-solana
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 20:38:25
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! Definitions of operations, types, utilities that can be shared among all Bangk's programs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_option::COption, pubkey::Pubkey,
};
use spl_token_2022::{
    extension::StateWithExtensions,
    state::{Account, Mint},
};

/// Definition of Custom Error for Bangk On-Chain programs
pub mod errors;
/// Definition of Bangk's PDAs
pub mod pda;
/// Definition of security checks perform before executing instructions
pub mod security;

/// Only output messages if in debug mode.
#[macro_export]
macro_rules! debug {
    ($($msg:expr),+$(,)?) => {
        #[cfg(feature = "debug-msg")]
        solana_program::msg!($($msg,)+)
    };
}

/// Get the owner of an account, be it a Mint, an ATA or client record.
/// If none of those, the account is assumed to be a PDA.
///
/// # Parameters
/// * `account` - Account for which to retrieve the owner
///
/// # Errors
/// If no valid owner as been found.
#[must_use]
pub fn get_account_owner(account: &AccountInfo) -> Pubkey {
    if let Ok(owner) = get_ata_owner(account) {
        return owner;
    }
    if let Ok(owner) = get_mint_owner(account) {
        return owner;
    }

    *account.owner
}

/// Get the owner of a given mint
///
/// # Parameters
/// * `mint` - Mint for which to retrieve the owner
///
/// # Errors
/// If the given account is not an (existing) mint
pub fn get_mint_owner(mint: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let state = StateWithExtensions::<Mint>::unpack(*mint.try_borrow_data()?)?.base;
    if let COption::Some(pubkey) = state.mint_authority {
        Ok(pubkey)
    } else {
        Err(ProgramError::InvalidArgument)
    }
}

/// Get the owner of a given ATA
///
/// # Parameters
/// * `ata` - ATA for which to retrieve the owner
///
/// # Errors
/// If the given account is not an (existing) ATA
pub fn get_ata_owner(ata: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let state = StateWithExtensions::<Account>::unpack(*ata.try_borrow_data()?)?.base;
    Ok(state.owner)
}

/// Get the current timestamp (or a close estimation to it)
///
/// # Errors
// If the clock could not be obtained
#[cfg(not(feature = "no-entrypoint"))]
pub fn get_timestamp() -> Result<i64, ProgramError> {
    use solana_program::{clock::Clock, sysvar::Sysvar as _};
    let clock = Clock::get()?;
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
pub fn get_timestamp() -> Result<i64, ProgramError> {
    Ok(chrono::Utc::now().timestamp())
}
