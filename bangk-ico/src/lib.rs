// File: bangk-ico/src/lib.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 14 August 2024 @ 19:18:29
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! Bangk's ICO On-Chain program.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod entrypoint;
mod instruction;
mod investment;
mod processor;
mod timelock;
mod unvesting;

// Only make public elements that would be useful.
/// The configuration PDA for Bangk's ICO program.
pub use config::ConfigurationPda;
/// Instructions for the Bangk ICO program.
pub use instruction::*;
/// Definition of a user's investment.
pub use investment::*;
/// Handles the dispatch of the processing operations (only used in tests).
pub use processor::process_instruction;
/// `Timelock` delay.
pub use processor::TIMELOCK_DELAY;
pub use timelock::TimelockPda;
/// Sets the rules for the unvesting.
pub use unvesting::*;

// Set the program's ID.
include!(concat!(env!("OUT_DIR"), "/program_id.rs"));

// Set the security.txt data
#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Bangk ICO Program",
    project_url: "https://www.bangk.app",
    contacts: "email:vincent.berthier@bangk.app",
    policy: "none at this time",

    // Optional
    preferred_languages: "fr,en"
}
