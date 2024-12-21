// File: bangk-stable/src/processor/mod.rs
// Project: bangk-onchain
// Creation date: Sunday 22 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:54:24
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

mod admin;
mod main;

pub use main::{process_instruction, INIT_KEY};
