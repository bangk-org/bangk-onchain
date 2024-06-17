// File: bangk-onchain-common/src/lib.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 24 July 2024 @ 21:43:30
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! Definitions of operations, types, utilities that can be shared among all Bangk's programs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Definition of Custom Error for Bangk On-Chain programs
mod error;
/// Query the state of the blockchain.
mod query;

/// Definition of Bangk's PDAs
pub mod pda;
/// Definition of security checks perform before executing instructions
pub mod security;

/// Re-export for Results and errors.
pub use crate::error::{Error, Result};
pub use query::*;

/// Only output messages if in debug mode.
#[macro_export]
macro_rules! debug {
    ($($msg:expr),+$(,)?) => {
        #[cfg(feature = "debug-msg")]
        solana_program::msg!($($msg,)+)
    };
}
