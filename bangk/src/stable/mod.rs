// File: bangk/src/stable/mod.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/stable/mod.rs
// Project: bangk-onchain
// Creation date: Thursday 23 November 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 25 March 2024 @ 16:28:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

//! The Solana On-Chain program's module for Bangk's stable operations.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Allows to burn stable coins from a client's account.
pub mod burn;
/// Exchange stable coins from one currency to another
pub mod exchange;
/// Add stable coins to a client's account.
pub mod mint;
/// Handle the transfer of tokens.
pub mod transfer;
