// File: bangk/src/lib.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/lib.rs
// Project: bangk-onchain
// Creation date: Friday 08 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::map_err_ignore)]

//! Bangk's Solana On-Chain program.

/// Program's entry point
mod entrypoint;
/// Definition of the different payloads triggering operations.
pub mod instruction;
/// Investment module handling Bangk's security tokens (Bangk Invest).
pub mod invest;
/// Handles the dispatch of the processing operations.
pub mod processor;
/// Stable Coin module handling Bangk's stable coins operations.
pub mod stable;
/// Data structures stored in accounts.
pub mod state;
/// Modules regrouping common operations.
pub mod utils;

// Set the program's ID.
include!(concat!(env!("OUT_DIR"), "/program_id.rs"));

// Set the security.txt data
#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Bangk Main Program",
    project_url: "https://www.bangk.app",
    contacts: "email:vincent.berthier@bangk.app",
    policy: "none at this time",

    // Optional
    preferred_languages: "fr,en"
}
