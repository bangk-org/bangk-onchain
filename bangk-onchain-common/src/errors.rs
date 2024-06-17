// File: bangk-onchain-common/src/errors.rs
// Project: bangk-solana
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 20:36:50
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use std::error::Error;
use thiserror::Error;

/// Custom error that can occur in a Bangk On-Chain Program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum BangkError {
    /// If attempting to create an account that already exists.
    #[error("the account already exists and can't be created")]
    AccountAlreadyExists,
    /// Tried to perform an operation on an account with the wrong owner.
    #[error("the owner of two related accounts doesn't match")]
    AccountOwnerMismatch,
    /// Given argument was too long.
    #[error("argument too long")]
    ArgumentTooLong,
    /// A computation could not be performed successfully.
    #[error("arithmetic error")]
    ArithmeticError,
    /// Tried to perform token operations on an account that doesn't exist yet.
    #[error("the ATA does not exist yet")]
    ATADoesNotExist,
    /// Tried to overwrite the BGK launch date.
    #[error("BGK token launch date has already been set")]
    BGKTokenAlreadyLaunched,
    /// Tried to cancel an ICO investment after the BGK launch.
    #[error("cannot cancel ICO investment after launch")]
    CancelIcoInvestmentAfterLaunch,
    /// Tried to close a mint with tokens in circulation.
    #[error("cannot close a mint if it still has tokens in circulation")]
    CannotCloseMintWithSupply,
    /// A CPI call has failed.
    #[error("an error happpened during a cross-program call")]
    CrossProgramCallFailed,
    /// Tried to pay dividends too soon.
    #[error("the next dividends payment date is in the future")]
    DividendPaymentsTriggeredTooSoon,
    /// Cannot invest in the ICO after the token's launch
    #[error("tried to create a new investment after the token launch")]
    IcoInvestAfterLaunch,
    /// Cannot unvest tokens before the launch.
    #[error("tried to unvest before the token launch")]
    IcoUnvestBeforeLaunch,
    /// Not enough tokens in the Bangk Exchange for this exchange operation.
    #[error("there are not enough tokens on the exchange ATA to transfer")]
    InsufficientExchangeFunds,
    /// Not enough tokens to perform the transfer.
    #[error("there are not enough tokens for this operation")]
    InsufficientFunds,
    /// The given amount is invalid (likely lower or equal to zero).
    #[error("the amount must be strictly greater than zero")]
    InvalidAmount,
    /// An ATA given does not match what was expected (wrong owner for example).
    #[error("an ATA does not match what was expected")]
    InvalidAta,
    /// Invalid exchange rate (must be greater than zero).
    #[error("the exchange rate must be strictly greater than zero")]
    InvalidExchangeRate,
    /// Invalid freeze / unfreeze status.
    #[error("the freeze / unfreeze status is invalid")]
    InvalidFreezeStatus,
    /// The given PDA is not own by this program.
    #[error("a PDA's owner is not Bangk's program")]
    InvalidOwner,
    /// The given PDA has the wrong type.
    #[error("the PDA account is not of the right type")]
    InvalidPdaType,
    /// The program ID is invalid.
    #[error("a program ID does not match the expected one.")]
    InvalidProgramId,
    /// Project configuration argument is not valid.
    #[error("invalid project argument")]
    InvalidProjectArgument,
    /// Tried to perform an operation on a project with the wrong status.
    #[error("the project's status does not allow this operation")]
    InvalidProjectStatus,
    /// There was an error when serializing or deserializing the data.
    #[error("data could not be (de)serialized as expected")]
    InvalidRawData,
    /// Instruction performed with wrong signers (not enough or unauthorized).
    #[error("signer is not authorized for this operation")]
    InvalidSigner,
    /// Invalid unvesting arguments (not enough or duplicates).
    #[error("invalid unvesting definition")]
    InvalidUnvestingDefinition,
    /// The given ATA does not match the Mint.
    #[error("the ATA is not associated to the given mint")]
    MismatchATAMint,
    /// The user's investment record does not match the given project.
    #[error("the record does not match the current project")]
    MismatchRecordProject,
    /// Missing a PDA account.
    #[error("PDA account info is missing")]
    MissingPDAAccount,
    /// The interest rates cannot be equal to zero.
    #[error("interest rate must be strictly greater than zero")]
    NegativeOrNullInterestRate,
    /// The `MultiSig` shouldn't have less than a given amount of keys.
    #[error("impossible to remove key from MultiSig as it would go below threshold")]
    NotEnoughMultiSigKeys,
    /// There was an integer overflow (one parameter is likely wrong).
    #[error("integer overflow detected")]
    IntegerOverflow,
    /// New project investments are only possible once the payment of dividends has finished.
    #[error("operation impossible: there are payments pending, try again later")]
    PendingPayments,
    /// The project has already been initialized.
    #[error("the project has already been initialized")]
    ProjectAlreadyInitialized,
    /// The rent exemption could not be retrieved from an account to close.
    #[error("the rent exemption couldn't be retrieved")]
    RentExemptionRetrieval,
    /// The ATAs are for the same currencies: there's no need to perform an exchange.
    #[error("target and source currencies are the same: no exchange necessary")]
    UnecessaryExchange,
    /// The given mint does not exist.
    #[error("the mint for the desired currency has not been initialized")]
    UnknownCurrency,
    /// An unknown error has occurred (should not happen obviously, check the logs…)
    #[error("unknown error")]
    UnknownError,
}

impl From<BangkError> for ProgramError {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from(err: BangkError) -> Self {
        msg!("BangkError: {}", err);
        Self::Custom(err as u32)
    }
}

impl From<u32> for BangkError {
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[allow(clippy::cognitive_complexity)]
    fn from(value: u32) -> Self {
        match value {
            x if x == Self::AccountAlreadyExists as u32 => Self::AccountAlreadyExists,
            x if x == Self::AccountOwnerMismatch as u32 => Self::AccountOwnerMismatch,
            x if x == Self::ArithmeticError as u32 => Self::ArithmeticError,
            x if x == Self::ArgumentTooLong as u32 => Self::ArgumentTooLong,
            x if x == Self::ATADoesNotExist as u32 => Self::ATADoesNotExist,
            x if x == Self::BGKTokenAlreadyLaunched as u32 => Self::BGKTokenAlreadyLaunched,
            x if x == Self::CancelIcoInvestmentAfterLaunch as u32 => {
                Self::CancelIcoInvestmentAfterLaunch
            }
            x if x == Self::CannotCloseMintWithSupply as u32 => Self::CannotCloseMintWithSupply,
            x if x == Self::CrossProgramCallFailed as u32 => Self::CrossProgramCallFailed,
            x if x == Self::DividendPaymentsTriggeredTooSoon as u32 => {
                Self::DividendPaymentsTriggeredTooSoon
            }
            x if x == Self::IcoInvestAfterLaunch as u32 => Self::IcoInvestAfterLaunch,
            x if x == Self::IcoUnvestBeforeLaunch as u32 => Self::IcoUnvestBeforeLaunch,
            x if x == Self::InsufficientExchangeFunds as u32 => Self::InsufficientExchangeFunds,
            x if x == Self::InsufficientFunds as u32 => Self::InsufficientFunds,
            x if x == Self::InvalidAmount as u32 => Self::InvalidAmount,
            x if x == Self::InvalidAta as u32 => Self::InvalidAta,
            x if x == Self::InvalidExchangeRate as u32 => Self::InvalidExchangeRate,
            x if x == Self::InvalidFreezeStatus as u32 => Self::InvalidFreezeStatus,
            x if x == Self::InvalidOwner as u32 => Self::InvalidOwner,
            x if x == Self::InvalidProjectArgument as u32 => Self::InvalidProjectArgument,
            x if x == Self::InvalidRawData as u32 => Self::InvalidRawData,
            x if x == Self::InvalidSigner as u32 => Self::InvalidSigner,
            x if x == Self::InvalidUnvestingDefinition as u32 => Self::InvalidUnvestingDefinition,
            x if x == Self::MismatchATAMint as u32 => Self::MismatchATAMint,
            x if x == Self::MismatchRecordProject as u32 => Self::MismatchRecordProject,
            x if x == Self::MissingPDAAccount as u32 => Self::MissingPDAAccount,
            x if x == Self::NegativeOrNullInterestRate as u32 => Self::NegativeOrNullInterestRate,
            x if x == Self::NotEnoughMultiSigKeys as u32 => Self::NotEnoughMultiSigKeys,
            x if x == Self::IntegerOverflow as u32 => Self::IntegerOverflow,
            x if x == Self::PendingPayments as u32 => Self::PendingPayments,
            x if x == Self::ProjectAlreadyInitialized as u32 => Self::ProjectAlreadyInitialized,
            x if x == Self::RentExemptionRetrieval as u32 => Self::RentExemptionRetrieval,
            x if x == Self::UnecessaryExchange as u32 => Self::UnecessaryExchange,
            x if x == Self::UnknownCurrency as u32 => Self::UnknownCurrency,
            x if x == Self::InvalidPdaType as u32 => Self::InvalidPdaType,
            x if x == Self::InvalidProgramId as u32 => Self::InvalidProgramId,
            x if x == Self::InvalidProjectStatus as u32 => Self::InvalidProjectStatus,
            _ => Self::UnknownError,
        }
    }
}

impl<T> DecodeError<T> for BangkError {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn type_of() -> &'static str {
        "BangkError"
    }
}

impl PrintProgramError for BangkError {
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[allow(clippy::too_many_lines)]
    fn print<E>(&self)
    where
        E: 'static + Error + DecodeError<E> + PrintProgramError + num_traits::FromPrimitive,
    {
        match *self {
            Self::AccountAlreadyExists => {
                msg!("Error: the account already exists and can't be created");
            }
            Self::AccountOwnerMismatch => {
                msg!("Error: the owner of two related accounts doesn't match");
            }
            Self::ArithmeticError => {
                msg!("Error: arithmetic error");
            }
            Self::ArgumentTooLong => {
                msg!("Error: one of the given arguments is too long");
            }
            Self::ATADoesNotExist => {
                msg!("Error: the ATA does not exist yet");
            }
            Self::BGKTokenAlreadyLaunched => {
                msg!("Error: BGK token launch date has already been set");
            }
            Self::CancelIcoInvestmentAfterLaunch => {
                msg!("Error: cannot cancel ICO investment after launch");
            }
            Self::CannotCloseMintWithSupply => {
                msg!("cannot close a mint if it still has tokens in circulation");
            }
            Self::CrossProgramCallFailed => {
                msg!("Error: an error happpened during a cross-program call");
            }
            Self::DividendPaymentsTriggeredTooSoon => {
                msg!("Error: the next dividends payment date is in the future");
            }
            Self::IcoInvestAfterLaunch => {
                msg!("Error: tried to create a new investment after the token launch");
            }
            Self::IcoUnvestBeforeLaunch => {
                msg!("Error: tried to unvest before the token launch");
            }
            Self::InsufficientExchangeFunds => {
                msg!("Error: there are not enough tokens on the exchange ATA to transfer");
            }
            Self::InsufficientFunds => {
                msg!("Error: there are not enough tokens to transfer");
            }
            Self::IntegerOverflow => {
                msg!("Error: integer overflow detected");
            }
            Self::InvalidAmount => {
                msg!("Error: the amount must be strictly greater than zero");
            }
            Self::InvalidAta => {
                msg!("Error: an ATA does not match what was expected");
            }
            Self::InvalidExchangeRate => {
                msg!("Error: the exchange rate must be strictly greater than zero");
            }
            Self::InvalidFreezeStatus => {
                msg!("Error: the freeze / unfreeze status is invalid");
            }
            Self::InvalidOwner => {
                msg!("Error: a PDA's owner is not Bangk's program");
            }
            Self::InvalidProjectArgument => {
                msg!("Error: invalid project argument");
            }
            Self::InvalidRawData => {
                msg!("Error: data could not be (de)serialized as expected");
            }
            Self::InvalidSigner => {
                msg!("Error: signer is not authorized for this operation");
            }
            Self::InvalidUnvestingDefinition => {
                msg!("Error: invalid unvesting definition");
            }
            Self::MismatchATAMint => {
                msg!("Error: the ATA is not associated to the given mint");
            }
            Self::MismatchRecordProject => {
                msg!("Error: the record does not match the current project");
            }
            Self::MissingPDAAccount => {
                msg!("Error: PDA account info is missing");
            }
            Self::NegativeOrNullInterestRate => {
                msg!("Error: interest rate must be strictly greater than zero");
            }
            Self::NotEnoughMultiSigKeys => {
                msg!(
                    "Error: impossible to remove key from MultiSig as it would go below threshold"
                );
            }
            Self::PendingPayments => {
                msg!("Error: operation impossible: there are payments pending, try again later");
            }
            Self::ProjectAlreadyInitialized => {
                msg!("Error: the project has already been initialized");
            }
            Self::RentExemptionRetrieval => {
                msg!("Error: the rent exemption couldn't be retrieved");
            }
            Self::UnecessaryExchange => {
                msg!("Error: target and source currencies are the same: no exchange necessary");
            }
            Self::UnknownCurrency => {
                msg!("Error: the mint for the desired currency has not been initialized");
            }
            Self::UnknownError => {
                msg!("Error: unknown error");
            }
            Self::InvalidPdaType => {
                msg!("Error: the PDA account is not of the right type");
            }
            Self::InvalidProgramId => {
                msg!("Error: a program ID does not match the expected one.");
            }
            Self::InvalidProjectStatus => {
                msg!("Error: the project's status does not allow this operation");
            }
        }
    }
}
