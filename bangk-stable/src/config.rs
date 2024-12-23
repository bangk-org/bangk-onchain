// File: bangk-stable/src/config.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 23 December 2024 @ 17:27:41
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::collections::HashMap;

use bangk_macro::pda;
use bangk_onchain_common::{
    pda::{BangkPda, PdaType},
    Error,
};
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
    /// Exchange rates from euro (or EUB) to foreign currencies
    pub exchange_rates: HashMap<String, f64>,
}

impl<'a> ConfigurationPda<'a> {
    /// Creates a new configuration PDA
    #[must_use]
    pub fn new(bump: u8, admin: &Pubkey) -> Self {
        Self {
            pda_type: Self::PDA_TYPE,
            bump,
            account: None,
            admin_multisig: *admin,
            launch_date: 0,
            amount_invested: 0,
            exchange_rates: HashMap::new(),
        }
    }

    /// Get the exchange rate from one currency to another
    ///
    /// # Parameters
    /// * `source` - Source currency,
    /// * `target` - Target currency.
    ///
    /// # Errors
    /// If the exchange rate could not be computed (missing data)
    pub fn get_exchange_rate(&self, source: &str, target: &str) -> Result<f64, Error> {
        if source == "EUB" {
            let rate = self
                .exchange_rates
                .get(target)
                .ok_or(Error::InvalidExchangeRate)?;
            return Ok(*rate);
        }

        if target == "EUB" {
            let rate = self
                .exchange_rates
                .get(target)
                .ok_or(Error::InvalidExchangeRate)?;
            let rate = 1.0_f64 / rate;
            return Ok(rate);
        }

        let source_to_eub = 1.0_f64
            / self
                .exchange_rates
                .get(source)
                .ok_or(Error::InvalidExchangeRate)?;
        let eub_to_target = *self
            .exchange_rates
            .get(target)
            .ok_or(Error::InvalidExchangeRate)?;
        let rate = source_to_eub * eub_to_target;
        Ok(rate)
    }
}
