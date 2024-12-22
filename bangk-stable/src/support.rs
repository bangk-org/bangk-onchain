// File: bangk-stable/src/support.rs
// Project: bangk-onchain
// Creation date: Sunday 22 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:55:18
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::Error;
use solana_program::{account_info::AccountInfo, program_error::ProgramError};
use spl_token_2022::{extension::StateWithExtensions, state::Mint};

/// Get the number of decimals used by a given stable coin
///
/// # Parameters
/// * `mint` - The account of the mint
#[inline]
fn get_decimals(mint: &AccountInfo) -> Result<u8, ProgramError> {
    get_mint_base_state(mint).map(|state| state.decimals)
}

/// Get the base state of a Mint
///
/// # Parameters
/// * `mint` - The account of the mint
pub fn get_mint_base_state(mint: &AccountInfo) -> Result<Mint, ProgramError> {
    Ok(StateWithExtensions::<Mint>::unpack(
        &mint
            .try_borrow_data()
            .map_err(|_err| Error::InvalidRawData)?,
    )
    .map_err(|_err| Error::InvalidRawData)?
    .base)
}

/// Get the number of tokens matching a given mint and amount
///
/// # Parameters
/// * `mint` - The mint for which to compute the number of tokens,
/// * `amount` - The raw amount (in 'full' stable coins).
///
/// # Errors
/// If the state of the mint could not be retrieved (if the account is not a mint for example)
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
pub fn get_token_amount(mint: &AccountInfo, amount: f64) -> Result<u64, ProgramError> {
    Ok((amount * 10_f64.powi(i32::from(get_decimals(mint)?))).floor() as u64)
}
