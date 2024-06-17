// File: bangk-ico/src/lib.rs
// Project: bangk-solana
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 20:35:38
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! Bangk's ICO On-Chain program.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

/// Program configuration.
pub mod config;
/// Program's entry point
mod entrypoint;
/// Definition of the different payloads triggering operations.
pub mod instruction;
/// Definition of a user's investment.
pub mod investment;
/// Handles the dispatch of the processing operations.
pub mod processor;
/// Sets the rules for the unvesting.
pub mod unvesting;

// Set the program's ID.
include!(concat!(env!("OUT_DIR"), "/program_id.rs"));

// Set the security.txt data
#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Bangk ICO Program",
    project_url: "https://www.bangk.app",
    contacts: "email:vincent.berthier@seven-france.net",
    policy: "none at this time",

    // Optional
    preferred_languages: "fr,en"
}
