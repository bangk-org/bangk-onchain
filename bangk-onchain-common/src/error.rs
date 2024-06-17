// File: bangk-onchain-common/src/error.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 16:38:06
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::{error, result};

use derive_more::{Display, From};
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};

/// Results for Bangk's programs with an `Error` associated by default.
pub type Result<T> = result::Result<T, Error>;

/// Custom error that can occur in a Bangk On-Chain Program
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub enum Error {
    /// If attempting to create an account that already exists.
    #[display("the account already exists and can't be created")]
    AccountAlreadyExists,
    /// Tried to perform an operation on an account with the wrong owner.
    #[display("the owner of two related accounts doesn't match")]
    AccountOwnerMismatch,
    /// Given argument was too long.
    #[display("argument too long")]
    ArgumentTooLong,
    /// A computation could not be performed successfully.
    #[display("arithmetic error")]
    ArithmeticError,
    /// Tried to perform token operations on an account that doesn't exist yet.
    #[display("the ATA does not exist yet")]
    ATADoesNotExist,
    /// Tried to overwrite the BGK launch date.
    #[display("BGK token launch date has already been set")]
    BGKTokenAlreadyLaunched,
    /// Tried to cancel an ICO investment after the BGK launch.
    #[display("cannot cancel ICO investment after launch")]
    CancelIcoInvestmentAfterLaunch,
    /// Tried to close a mint with tokens in circulation.
    #[display("cannot close a mint if it still has tokens in circulation")]
    CannotCloseMintWithSupply,
    /// Could not obtain the clock.
    #[display("failed to obtain the clock from the blockchain")]
    Clock,
    /// A CPI call has failed.
    #[display("an error happpened during a cross-program call")]
    CrossProgramCallFailed,
    /// Tried to pay dividends too soon.
    #[display("the next dividends payment date is in the future")]
    DividendPaymentsTriggeredTooSoon,
    /// A security key for `MultiSig` is duplicated.
    #[display("duplicated key in multisig definition")]
    DuplicatedKeyInMultisigDefinition,
    /// Cannot invest in the ICO after the token's launch
    #[display("tried to create a new investment after the token launch")]
    IcoInvestAfterLaunch,
    /// Cannot unvest tokens before the launch.
    #[display("tried to unvest before the token launch")]
    IcoUnvestBeforeLaunch,
    /// Not enough tokens in the Bangk Exchange for this exchange operation.
    #[display("there are not enough tokens on the exchange ATA to transfer")]
    InsufficientExchangeFunds,
    /// Not enough tokens to perform the transfer.
    #[display("there are not enough tokens for this operation")]
    InsufficientFunds,
    /// The given amount is invalid (likely lower or equal to zero).
    #[display("the amount must be strictly greater than zero")]
    InvalidAmount,
    /// An ATA given does not match what was expected (wrong owner for example).
    #[display("an ATA does not match what was expected")]
    InvalidAta,
    /// The data of an account does not match what's expected from an ATA.
    #[display("invalid data for ATA")]
    InvalidAtaData,
    /// Invalid exchange rate (must be greater than zero).
    #[display("the exchange rate must be strictly greater than zero")]
    InvalidExchangeRate,
    /// Invalid freeze / unfreeze status.
    #[display("the freeze / unfreeze status is invalid")]
    InvalidFreezeStatus,
    /// The given PDA is not own by this program.
    #[display("a PDA's owner is not Bangk's program")]
    InvalidOwner,
    /// The given PDA has the wrong type.
    #[display("the PDA account is not of the right type")]
    InvalidPdaType,
    /// The program ID is invalid.
    #[display("a program ID does not match the expected one.")]
    InvalidProgramId,
    /// Project configuration argument is not valid.
    #[display("invalid project argument")]
    InvalidProjectArgument,
    /// Tried to perform an operation on a project with the wrong status.
    #[display("the project's status does not allow this operation")]
    InvalidProjectStatus,
    /// There was an error when serializing or deserializing the data.
    #[display("data could not be (de)serialized as expected")]
    InvalidRawData,
    /// Instruction performed with wrong signers (not enough or unauthorized).
    #[display("signer is not authorized for this operation")]
    InvalidSigner,
    /// Invalid unvesting arguments (not enough or duplicates).
    #[display("invalid unvesting definition")]
    InvalidUnvestingDefinition,
    /// The targeted investment does not exist.
    #[display("the investment does not exist")]
    InvestmentDoesNotExist,
    /// The given ATA does not match the Mint.
    #[display("the ATA is not associated to the given mint")]
    MismatchATAMint,
    /// The user's investment record does not match the given project.
    #[display("the record does not match the current project")]
    MismatchRecordProject,
    /// Missing a PDA account.
    #[display("PDA account info is missing")]
    MissingPDAAccount,
    /// The interest rates cannot be equal to zero.
    #[display("interest rate must be strictly greater than zero")]
    NegativeOrNullInterestRate,
    /// The `MultiSig` shouldn't have less than a given amount of keys.
    #[display("impossible to remove key from MultiSig as it would go below threshold")]
    NotEnoughMultiSigKeys,
    /// There was an integer overflow (one parameter is likely wrong).
    #[display("integer overflow detected")]
    IntegerOverflow,
    /// New project investments are only possible once the payment of dividends has finished.
    #[display("operation impossible: there are payments pending, try again later")]
    PendingPayments,
    /// The project has already been initialized.
    #[display("the project has already been initialized")]
    ProjectAlreadyInitialized,
    /// The rent exemption could not be retrieved from an account to close.
    #[display("the rent exemption couldn't be retrieved")]
    RentExemptionRetrieval,
    /// The ATAs are for the same currencies: there's no need to perform an exchange.
    #[display("target and source currencies are the same: no exchange necessary")]
    UnecessaryExchange,
    /// The current instruction can only be run once and has already been executed.
    #[display("unique operation already executed")]
    UniqueOperationAlreadyExecuted,
    /// The given mint does not exist.
    #[display("the mint for the desired currency has not been initialized")]
    UnknownCurrency,
    /// An unknown error has occurred (should not happen obviously, check the logs…)
    #[display("unknown error")]
    UnknownError,
}

impl From<Error> for ProgramError {
    fn from(err: Error) -> Self {
        msg!("BangkError: {}", err);
        Self::Custom(err as u32)
    }
}

impl From<u32> for Error {
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
            x if x == Self::DuplicatedKeyInMultisigDefinition as u32 => {
                Self::DuplicatedKeyInMultisigDefinition
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
            x if x == Self::UniqueOperationAlreadyExecuted as u32 => {
                Self::UniqueOperationAlreadyExecuted
            }
            x if x == Self::UnknownCurrency as u32 => Self::UnknownCurrency,
            x if x == Self::InvalidPdaType as u32 => Self::InvalidPdaType,
            x if x == Self::InvalidProgramId as u32 => Self::InvalidProgramId,
            x if x == Self::InvalidProjectStatus as u32 => Self::InvalidProjectStatus,
            _ => Self::UnknownError,
        }
    }
}

impl error::Error for Error {}

impl<T> DecodeError<T> for Error {
    fn type_of() -> &'static str {
        "BangkError"
    }
}

impl PrintProgramError for Error {
    fn print<E>(&self)
    where
        E: 'static + error::Error + DecodeError<E> + PrintProgramError + num_traits::FromPrimitive,
    {
        msg!("BangkError: {}", self);
    }
}
