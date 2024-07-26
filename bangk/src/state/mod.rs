// File: bangk/src/state/mod.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/mod.rs
// Project: bangk-onchain
// Creation date: Wednesday 10 January 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 16 July 2024 @ 16:49:55
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::Error;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke,
    program_error::ProgramError, pubkey::Pubkey,
};
use spl_token_2022::{extension::StateWithExtensions, state::Mint};
use spl_token_metadata_interface::{
    borsh::BorshDeserialize as _,
    instruction::update_field,
    state::{Field, TokenMetadata},
};

use crate::processor::DELEGATE;

/// Index of the start of the metadata information in a mint account.
pub const MINT_META_START: usize = 279;

/// Client account data.
pub mod clients;
/// Tracks the state of the payment of dividends.
pub mod dividends_tracker;
/// Common trait for all mints.
pub mod mint_data;
/// Common mints definitions.
pub mod mints;
/// Bangk' PDA definition.
pub mod pda;
/// Investment projects account data.
pub mod projects;
/// Stable mints used by Bangk.
pub mod stable;

/// Create a new Token Metadata.
///
/// # Parameters
/// * `mint` - Mint the metadata are attached to.
/// * `name` - Name of the token.
/// * `symbol` - Symbol of the token.
/// * `uri` - `URI` referring to the token.
/// * `additional_metadata` - Key/Value pairs of additional metadata
///   to attach to the token.
///
/// # Errors
/// If the update authority could not be set.
pub fn token_metadata<T, S, R, Q>(
    mint: &Pubkey,
    name: T,
    symbol: S,
    uri: R,
    additional_metadata: Q,
) -> Result<TokenMetadata, ProgramError>
where
    T: Into<String>,
    S: Into<String>,
    R: Into<String>,
    Q: Into<Vec<(String, String)>>,
{
    Ok(TokenMetadata {
        update_authority: Some(DELEGATE).try_into()?,
        mint: *mint,
        name: name.into(),
        symbol: symbol.into(),
        uri: uri.into(),
        additional_metadata: additional_metadata.into(),
    })
}

/// Get the Token Metadata for a given mint.
///
/// # Parameters
/// * `mint` - Mint for which to get the metadata.
///
/// # Returns
/// * The metadata associated with the mint.
///
/// # Errors
/// If the mint's data can not be deserialized.
pub fn get_mint_metadata(mint: &AccountInfo) -> Result<TokenMetadata, ProgramError> {
    let data = &mint.data.borrow();
    TokenMetadata::try_from_slice(data.get(MINT_META_START..).ok_or(Error::InvalidRawData)?)
        .map_err(|_| ProgramError::InvalidAccountData)
}

/// Updates one metadata field.
///
/// # Parameters
/// * `owner` - Owner of the mint and fee payer.
/// * `mint` - Account of the mint for which the metadata are updated.
/// * `key` - Key of the field to update
/// * `value` - Value of the metadata to update.
///
/// # Errors
/// Never.
pub fn update_metadata_field<'a>(
    bangk: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    key: Field,
    value: &str,
) -> ProgramResult {
    invoke(
        &update_field(
            &spl_token_2022::id(),
            mint.key,
            bangk.key,
            key,
            value.to_owned(),
        ),
        &[mint.clone(), bangk.clone()],
    )
}

/// Get the mint's state.
///
/// # Parameters
/// * `account` - Account of the mint.
///
/// # Returns
/// * Mint base state.
///
/// # Errors
/// If the mint's data can not be deserialized.
#[inline]
pub fn get_state(account: &AccountInfo) -> Result<Mint, ProgramError> {
    Ok(StateWithExtensions::<Mint>::unpack(
        &account
            .try_borrow_data()
            .map_err(|_| Error::InvalidRawData)?,
    )
    .map_err(|_| Error::InvalidRawData)?
    .base)
}
