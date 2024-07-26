// File: bangk/src/utils/mod.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/utils/mod.rs
// Project: bangk-onchain
// Creation date: Friday 24 November 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:46:35
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! Utilities modules for Bangk's On-Chain program.

use std::str::FromStr;

use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::state::{mint_data::MintData as _, stable::StableMint};

/// Handles all the operations performed on a project.
pub mod accounts;
/// Common operations for tokens.
pub mod tokens;

/// Transforms a string into a public key if possible.
#[must_use]
#[inline]
pub fn to_key(key: &str) -> Option<Pubkey> {
    Pubkey::from_str(key).ok()
}

/// Checks that accounts are associated to the right mint.
///
/// # Parameters
/// * `mint` - Mint supposedly associated to the token accounts.
/// * `accounts` - Accounts to check.
#[macro_export]
macro_rules! check_mint_ata {
    ($mint:ident, $ata:ident) => {
        let associated_mint = StateWithExtensions::<Account>::unpack(*$ata.try_borrow_data()?)?
            .base
            .mint;
        if *$mint.key != associated_mint {
            return Err(bangk_onchain_common::Error::MismatchATAMint.into());
        }
    };
    ($mint:ident, $ata:ident, $($tail:ident),*) => {
        check_mint_ata!($mint, $ata);
        check_mint_ata!($mint, $($tail)*);
    }
}

/// Check that a mint, a client record and his stable ATA are all as expected.
#[macro_export]
macro_rules! check_investment {
    ($mint:ident, $record:ident, $ata:ident) => {
        if $record.lamports() > 0 {
            // Parse the record
            let record =
                $crate::state::pda::from_account::<$crate::state::clients::Investment>($record)?;
            if $crate::utils::to_key(&record.ata).unwrap_or_default() != *$ata.key {
                bangk_onchain_common::debug!(
                    "mismatch between record and ata: expected {}, got {}.",
                    record.ata,
                    $ata.key
                );
                return Err(bangk_onchain_common::Error::InvalidAta.into());
            }
            if $crate::utils::to_key(&record.project).unwrap_or_default() != *$mint.key {
                return Err(bangk_onchain_common::Error::MismatchRecordProject.into());
            }
        }
    };
}

/// Check that the project's status is the expected one.
#[macro_export]
macro_rules! check_project_status {
    ($mint:ident, $status:expr) => {
        let project: $crate::state::projects::Project =
            $crate::state::get_mint_metadata(&$mint)?.try_into()?;
        if project.status != $status {
            return Err(bangk_onchain_common::Error::InvalidProjectStatus.into());
        }
    };
}

/// Get the Address for the mint of a given stable coin
///
/// This function should not be used by the On Chain program.
///
/// # Parameters
/// * `currency` - Symbol of the stable coins
///
/// # Returns
/// * Tuple of public Key of the stable Coin and associated bump
#[must_use]
#[inline]
pub fn get_stable_mint_address(currency: &str) -> (Pubkey, u8) {
    StableMint::get_address(currency)
}

/// Get a client's ATA for a mint (stable or project).
///
/// This function should not be used by the On Chain program.
///
/// # Parameters
/// * `client` - ID of the client,
/// * `mint` - Mint the ATA is associated to.
///
/// # Returns
/// * ATA
#[must_use]
#[inline]
pub fn get_ata_address(client: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(client, mint, &spl_token_2022::id())
}
