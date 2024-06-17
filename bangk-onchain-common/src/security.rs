// File: bangk-onchain-common/src/security.rs
// Project: bangk-solana
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 20:35:38
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use borsh::{BorshDeserialize, BorshSerialize};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    errors::BangkError,
    pda::{BangkPda, PdaType},
};

/// Checks that given accounts are valid ATAs.
#[macro_export]
macro_rules! check_ata_exists {
    () => {};
    ($a:expr) => {
        if $a.lamports() == 0 {
            return Err(bangk_onchain_common::errors::BangkError::ATADoesNotExist.into());
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
        let pda: $crate::security::MultiSigPda = $crate::pda::from_account($multisig)?;
        pda.multisig.validate($accounts, $level)?;
    };
}

/// Check that the given program owns the PDA
#[macro_export]
macro_rules! check_pda_owner {
    ($program_id:ident, $pda:expr $(,)?) => {
        if $pda.lamports() > 0 && $pda.owner != $program_id {
            $crate::debug!("{} has owner {} and not {}", stringify!($pda), $pda.owner, $program_id);
            return Err(bangk_onchain_common::errors::BangkError::InvalidOwner.into());
        }
    };
    ($program_id:ident, $pda:expr $(, $tail:expr)*) => {
        check_pda_owner!($program_id, $pda);
        check_pda_owner!($program_id $(, $tail)*);
    }
}

/// Checks that given accounts all have the same owner
#[macro_export]
macro_rules! check_same_owner {
    ($a:ident) => {};
    ($a:ident, $b:ident) => {
        // either account can be a client investment PDA, or a token account
        // but we only care about the case in which both parses are ok
        if $crate::get_account_owner($a) != $crate::get_account_owner($b) {
            $crate::debug!("{} and {} do not belong to the same owner", stringify!($a), stringify!($b));
            return Err(bangk_onchain_common::errors::BangkError::AccountOwnerMismatch.into());
        }
    };
    ($a:ident, $b:ident, $($tail:ident),*) => {
        check_same_owner!($b, $($tail)*);
    };
}

/// Checks that given accounts all belong to the current program
#[macro_export]
macro_rules! check_bangk_owner {
    ($a:ident) => {};
    ($pid:ident, $a:ident) => {
        let owner = $crate::get_account_owner($a);
        if (owner != *$pid) {
            $crate::debug!("{} does not belong to Bangk", stringify!($a));
            return Err(bangk_onchain_common::errors::BangkError::AccountOwnerMismatch.into());
        }
    };
    ($pid:ident, $a:ident, $($tail:tt)*) => {
        check_bangk_owner!($pid, $a);
        check_bangk_owner!($pid, $($tail)*);
    };
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
            return Err(bangk_onchain_common::errors::BangkError::InvalidProgramId.into());
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
            return Err(bangk_onchain_common::errors::BangkError::InvalidProgramId.into());
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
            return Err(bangk_onchain_common::errors::BangkError::InvalidProgramId.into());
        }
    };
}

/// Type of the Multisig (admin, freeze, *etc.*)
#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum MultiSigType {
    /// Contains the list of keys allowed to sign Critical / Admin instructions.
    Admin,
    /// Contains the list of keys allowed to freeze or unfreeze ATAs.
    Freeze,
}

/// Defines the different levels of security attached to operations.
#[derive(Clone, Copy, Debug)]
pub enum OperationSecurityLevel {
    /// Routine operations only require one signer.
    Routine,
    /// Sensitive operations will require at least two signers.
    Sensitive,
    /// Critical operations will require at least three signers.
    Critical,
}

impl OperationSecurityLevel {
    /// Get the number of authorized keys required to validated an operation of the given level.
    ///
    /// # Returns
    /// The number of keys required to validate the operation.
    #[must_use]
    pub const fn required_keys(&self) -> u8 {
        match self {
            Self::Routine => 1,
            Self::Sensitive => 2,
            Self::Critical => 3,
        }
    }
}

/// Definition of a Multisig
#[derive(BorshSerialize, BorshDeserialize, Debug, ShankType)]
pub struct MultiSig {
    /// Type of the Multisig (admin, freeze, *etc.*)
    pub sig_type: MultiSigType,
    /// Keys belonging to the Multisig.
    pub keys: Vec<Pubkey>,
}

/// PDA for a Multisig.
#[derive(BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct MultiSigPda {
    /// Type of the PDA (Must be `PdaType::MultiSig`).
    pub pda_type: PdaType,
    /// Seed bump to obtain the PDA address.
    pub bump: u8,
    /// Multisig definition
    pub multisig: MultiSig,
}

impl MultiSig {
    /// Create a new `MultiSig` type.
    #[must_use]
    pub const fn new(sig_type: MultiSigType, keys: Vec<Pubkey>) -> Self {
        Self { sig_type, keys }
    }

    /// Checks that there are enough valid signatures for this Multisig
    ///
    /// # Parameters
    /// * `accounts` - Instruction accounts,
    /// * `sig_type` - Type of the `MultiSig` expected,
    /// * `level` - Security level of the operation.
    ///
    /// # Errors
    /// If there aren't enough valid signatures.
    pub fn validate(
        &self,
        accounts: &[AccountInfo],
        level: OperationSecurityLevel,
    ) -> ProgramResult {
        let n = level.required_keys();
        if accounts.len() >= n as usize
            && accounts
                .iter()
                .take(n as usize)
                .all(|account| account.is_signer && self.keys.contains(account.key))
        {
            Ok(())
        } else {
            Err(BangkError::InvalidSigner.into())
        }
    }
}

impl MultiSigPda {
    /// Create a new Account for the definition of Freeze Keys.
    ///
    /// # Parameters
    /// * `account` - Account where the PDA is stored,
    /// * `bump` - Bump used to derive the PDA address,
    /// * `multisig` - Definition of the `MultiSig`.
    #[must_use]
    pub const fn new(bump: u8, multisig: MultiSig) -> Self {
        Self {
            pda_type: PdaType::MultiSig,
            bump,
            multisig,
        }
    }

    /// Get the PDA's address.
    ///
    /// This function should **not** be used by the On Chain program.
    ///
    /// # Parameters
    /// * `sig_type` - Type of the `MultiSig`,
    /// * `program_id` - Program owning the PDA.
    ///
    /// # Returns
    /// * Tuple of public Key of the investment record and associated bump
    #[must_use]
    pub fn get_address(sig_type: MultiSigType, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"BangkMultiSig", &[sig_type as u8]], program_id)
    }
}

impl BangkPda for MultiSigPda {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn get_bump(&self) -> u8 {
        self.bump
    }

    fn is_valid(&self) -> bool {
        self.pda_type == PdaType::MultiSig
    }

    #[must_use]
    fn seeds(&self) -> Vec<Vec<u8>> {
        vec![
            b"BangkMultiSig".to_vec(),
            vec![self.multisig.sig_type as u8],
            vec![self.bump],
        ]
    }
}
