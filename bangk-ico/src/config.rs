// File: bangk-ico/src/config.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 12 August 2024 @ 16:48:10
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::collections::HashMap;

use bangk_macro::pda;
use bangk_onchain_common::pda::{BangkPda, PdaType};
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;

use crate::unvesting::{UnvestingScheme, UnvestingType};

/// Configuration PDA of the ICO program.
#[pda(kind = PdaType::ProgramConfiguration, seed = "Configuration")]
pub struct ConfigurationPda {
    /// Definition of the unvesting schemes.
    pub unvesting: HashMap<UnvestingType, UnvestingScheme>,
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
    pub fn new(bump: u8, unvesting: &[UnvestingScheme], admin: &Pubkey) -> Self {
        let mut map = HashMap::new();
        for def in unvesting {
            map.insert(def.kind, *def);
        }
        Self {
            pda_type: Self::PDA_TYPE,
            bump,
            account: None,
            unvesting: map,
            admin_multisig: *admin,
            launch_date: 0,
            amount_invested: 0,
        }
    }
}
