// File: bangk-stable/src/config.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:54:24
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_macro::pda;
use bangk_onchain_common::pda::{BangkPda, PdaType};
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;

/// Configuration PDA of the ICO program.
#[pda(kind = PdaType::ProgramConfiguration, seed = "Configuration")]
pub struct ConfigurationPda {
    /// Address of the PDA for the Admin `MultiSig`.
    pub admin_multisig: Pubkey,
    /// Date of the BGK launch.
    pub launch_date: i64,
    /// Amount of invested tokens
    pub amount_invested: u64,
}

impl<'a> ConfigurationPda<'a> {
    /// Creates a new configuration PDA
    #[must_use]
    pub const fn new(bump: u8, admin: &Pubkey) -> Self {
        Self {
            pda_type: Self::PDA_TYPE,
            bump,
            account: None,
            admin_multisig: *admin,
            launch_date: 0,
            amount_invested: 0,
        }
    }
}
