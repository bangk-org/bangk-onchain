// File: bangk-stable/src/lib.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 24 December 2024 @ 18:58:55
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! Bangk's ICO On-Chain program.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod entrypoint;
mod instruction;
mod processor;
mod support;

/// The configuration PDA for Bangk's ICO program.
pub use config::ConfigurationPda;
/// Instructions for the Bangk ICO program.
pub use instruction::*;
/// Handles the dispatch of the processing operations (only used in tests).
pub use processor::{process_instruction, INIT_KEY};
/// Support functions
pub use support::{compute_token_amount, get_decimals};

// Set the program's ID.
solana_program::declare_id!("BKPrg5rXBCXMEJPnL2K8DaFEcua1e1SkeLUGqSQhkj6U");

/// Seed used to compute the address of a Stable Coin Mint PDA
pub const STABLE_MINT_SEED: &str = "BangkStableCoin";
/// Seed used to compute the address of a Stable Coin Exchange PDA
pub const EXCHANGE_WALLET_SEED: &str = "ExchangeWallet";

// Set the security.txt data
#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Bangk Stable Coins Program",
    project_url: "https://www.bangk.app",
    contacts: "email:vincent.berthier@bangk.app",
    policy: "none at this time",

    // Optional
    preferred_languages: "fr,en"
}
