// File: bangk-stable/src/processor/mod.rs
// Project: bangk-onchain
// Creation date: Sunday 22 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 24 December 2024 @ 17:23:10
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

mod admin;
mod coins;
mod main;
mod supply;
mod transfers;

pub use main::{process_instruction, INIT_KEY};
