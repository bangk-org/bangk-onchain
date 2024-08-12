// File: bangk-onchain-common/src/security/multisig.rs
// Project: bangk-onchain
// Creation date: Thursday 25 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 12 August 2024 @ 16:45:53
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::collections::HashSet;

use bangk_macro::pda;
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    pda::{BangkPda, PdaType, Seed},
    Error,
};

/// Type of the `MultiSig` (admin, freeze, *etc.*)
#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum MultiSigType {
    /// Contains the list of keys allowed to sign Critical / Admin instructions.
    Admin,
    /// Contains the list of keys allowed to freeze or unfreeze ATAs.
    Freeze,
}

impl From<MultiSigType> for u8 {
    fn from(value: MultiSigType) -> Self {
        value as Self
    }
}

impl From<MultiSigType> for Seed {
    fn from(value: MultiSigType) -> Self {
        Self::from(value as u8)
    }
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

/// Definition of a `MultiSig`
#[derive(BorshSerialize, BorshDeserialize, Debug, ShankType)]
pub struct MultiSig {
    /// Type of the `MultiSig` (admin, freeze, *etc.*)
    pub sig_type: MultiSigType,
    /// Keys belonging to the `MultiSig`.
    pub keys: Vec<Pubkey>,
}

impl MultiSig {
    /// Create a new `MultiSig` type.
    #[must_use]
    pub const fn new(sig_type: MultiSigType, keys: Vec<Pubkey>) -> Self {
        Self { sig_type, keys }
    }

    /// Checks that there are enough valid signatures for this `MultiSig`
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
        let n = level.required_keys() as usize;

        // Get non-duplicated signers
        let signers = accounts
            .iter()
            .take(n)
            .map(|acc| (acc.is_signer, acc.key))
            .collect::<HashSet<_>>();
        if signers.len() < n {
            return Err(Error::InvalidSigner.into());
        }

        if signers
            .iter()
            .all(|(signer, key)| *signer && self.keys.contains(key))
        {
            Ok(())
        } else {
            Err(Error::InvalidSigner.into())
        }
    }
}

/// PDA for a `MultiSig`.
#[pda(kind = PdaType::MultiSig, seed = "Multisig", seed = multisig.sig_type)]
pub struct MultiSigPda {
    /// `MultiSig` definition
    pub multisig: MultiSig,
}

impl<'a> MultiSigPda<'a> {
    /// Create a new Account for the definition of Freeze Keys.
    ///
    /// # Parameters
    /// * `bump` - Bump used to derive the PDA address,
    /// * `multisig` - Definition of the `MultiSig`.
    #[must_use]
    pub const fn new(bump: u8, multisig: MultiSig) -> Self {
        Self {
            pda_type: Self::PDA_TYPE,
            bump,
            multisig,
            account: None,
        }
    }
}
