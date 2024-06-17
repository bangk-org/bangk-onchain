// File: bangk-onchain-common/src/security/checks.rs
// Project: bangk-onchain
// Creation date: Thursday 25 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 25 July 2024 @ 20:48:08
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

/// Checks that given accounts are valid ATAs.
#[macro_export]
macro_rules! check_ata_exists {
    () => {};
    ($a:expr) => {
        if $a.lamports() == 0 {
            return Err(bangk_onchain_common::Error::ATADoesNotExist.into());
        }
    };
    ($a:expr, $($tail:tt)*) => {
        check_ata_exists!($a);
        check_ata_exists!($($tail)*);
    };
}

/// Check that the instructions is signed correctly.
#[macro_export]
macro_rules! check_signers {
    // In the case where we have only two accounts, it's a routine operation
    ($accounts:expr, $multisig:expr) => {
        check_signers!(
            $accounts,
            $multisig,
            $crate::security::OperationSecurityLevel::Routine
        );
    };
    // Otherwise we need to validate the multisig
    ($accounts:expr, $multisig:expr, $level:path) => {
        let pda = $crate::security::MultiSigPda::from_account($multisig)?;
        pda.multisig.validate($accounts, $level)?;
    };
}

/// Check that the given program owns the PDA
#[macro_export]
macro_rules! check_pda_owner {
    ($program_id:ident, $pda:expr $(,)?) => {
        if $pda.lamports() > 0 && $pda.owner != $program_id {
            $crate::debug!("{} has owner {} and not {}", stringify!($pda), $pda.owner, $program_id);
            return Err(bangk_onchain_common::Error::InvalidOwner.into());
        }
    };
    ($program_id:ident, $pda:expr $(, $tail:expr)*) => {
        check_pda_owner!($program_id, $pda);
        check_pda_owner!($program_id $(, $tail)*);
    }
}

/// Checks that the given account's key matches the System program ID
///
/// # Arguments
/// * `account` - The account to check
///
/// # Errors
/// If the account's key does not match
#[macro_export]
macro_rules! check_system_program {
    ($id:ident) => {
        if *$id.key != solana_program::system_program::id() {
            return Err(bangk_onchain_common::Error::InvalidProgramId.into());
        }
    };
}

/// Checks that the given account's key matches the SPL Token 2022 program ID
///
/// # Arguments
/// * `account` - The account to check
///
/// # Errors
/// If the account's key does not match
#[macro_export]
macro_rules! check_spl_program {
    ($id:ident) => {
        if *$id.key != spl_token_2022::id() {
            return Err(bangk_onchain_common::Error::InvalidProgramId.into());
        }
    };
}

/// Checks that the given account's key matches the Associated Token Account program ID
///
/// # Arguments
/// * `account` - The account to check
///
/// # Errors
/// If the account's key does not match
#[macro_export]
macro_rules! check_ata_program {
    ($id:ident) => {
        if *$id.key != spl_associated_token_account::id() {
            return Err(bangk_onchain_common::Error::InvalidProgramId.into());
        }
    };
}
