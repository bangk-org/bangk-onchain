// File: bangk-ico/src/config.rs
// Project: bangk-solana
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Tuesday 25 June 2024 @ 15:48:27
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::collections::HashMap;

use bangk_onchain_common::pda::{BangkPda, PdaType};
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::pubkey::Pubkey;

use crate::unvesting::{UnvestingScheme, UnvestingType};

/// Configuration PDA of the ICO program.
#[derive(BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct IcoConfigurationPda {
    /// Type of the PDA (Must be `PdaType::MultiSig`).
    pub pda_type: PdaType,
    /// Seed bump to obtain the PDA address.
    pub bump: u8,
    /// Definition of the unvesting schemes.
    pub unvesting: HashMap<UnvestingType, UnvestingScheme>,
    /// Address of the PDA for the Admin `MultiSig`.
    pub admin_multisig: Pubkey,
    /// Date of the BGK launch.
    pub launch_date: i64,
}

impl IcoConfigurationPda {
    /// Creates a new configuration PDA
    #[must_use]
    pub fn new(bump: u8, unvesting: &[UnvestingScheme], admin: &Pubkey) -> Self {
        let mut map = HashMap::new();
        for def in unvesting {
            map.insert(def.kind, *def);
        }
        Self {
            pda_type: PdaType::ProgramConfiguration,
            bump,
            unvesting: map,
            admin_multisig: *admin,
            launch_date: 0,
        }
    }

    /// Get the PDA's address.
    ///
    /// This function should **not** be used by the On Chain program.
    ///
    /// # Parameters
    /// * `sig_type` - Type of the `MultiSig`,
    /// * `program_id` - Program owning the PDA.
    ///
    /// # Returns
    /// * Tuple of public Key of the investment record and associated bump
    #[must_use]
    pub fn get_address() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"IcoConfigurationPda"], &crate::ID)
    }
}

impl BangkPda for IcoConfigurationPda {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn get_bump(&self) -> u8 {
        self.bump
    }

    fn is_valid(&self) -> bool {
        self.pda_type == PdaType::ProgramConfiguration
    }

    #[must_use]
    fn seeds(&self) -> Vec<Vec<u8>> {
        vec![b"IcoConfigurationPda".to_vec(), vec![self.bump]]
    }
}
