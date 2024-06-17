// File: bangk-ico/src/investment.rs
// Project: bangk-solana
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 20:35:38
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{
    errors::BangkError,
    get_timestamp,
    pda::{BangkPda, PdaType},
};
use borsh::{BorshDeserialize, BorshSerialize};
use shank::{ShankAccount, ShankType};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

use crate::unvesting::{UnvestingScheme, UnvestingType};

/// Definition of a user's ICO investment.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq, ShankType)]
pub struct UserIcoInvestment {
    /// Type of unvesting.
    pub kind: UnvestingType,
    /// Timestamp at which the investment has been done.
    pub timestamp: i64,
    /// Custom rules of unvesting if necessary.
    pub custom_rule: Option<UnvestingScheme>,
    /// Number of tokens bought.
    pub amount_bought: u64,
    /// Number of tokens already released.
    pub amount_released: u64,
}

/// Stores the data for a user's investments.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct UserInvestment {
    /// User owning the investment.
    pub user: Pubkey,
    /// Definition of the investments from the user.
    pub investments: Vec<UserIcoInvestment>,
}

impl UserInvestment {
    /// Create a new user's investment.
    ///
    /// # Parameters
    /// * `user` - User owning the investment,
    /// * `kind` - Type of unvesting for the investment,
    /// * `amount` - Amount of tokens bought,
    /// * `custom_rule` - Custom unvesting rule if necessary.
    ///
    /// # Errors
    /// If the custom rule is given but invalid.
    pub fn new(
        user: Pubkey,
        kind: UnvestingType,
        amount: u64,
        custom_rule: Option<UnvestingScheme>,
    ) -> Result<Self, ProgramError> {
        match custom_rule {
            None => (),
            Some(rule) => {
                if rule.kind != kind || !rule.is_valid().unwrap_or(false) {
                    return Err(BangkError::InvalidUnvestingDefinition.into());
                }
            }
        }
        Ok(Self {
            user,
            investments: vec![UserIcoInvestment {
                kind,
                timestamp: get_timestamp()?,
                custom_rule,
                amount_bought: amount,
                amount_released: 0,
            }],
        })
    }
}

/// PDA for a `UserInvestment`.
#[derive(BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct UserInvestmentPda {
    /// Type of the PDA (Must be `PdaType::MultiSig`).
    pub pda_type: PdaType,
    /// Seed bump to obtain the PDA address.
    pub bump: u8,
    /// Investment data
    pub investment: UserInvestment,
}

impl UserInvestmentPda {
    /// Create a new Account for the definition of Freeze Keys.
    ///
    /// # Parameters
    /// * `bump` - Bump used to derive the PDA address,
    /// * `investment` - Definition of the `UserInvestment`.
    #[must_use]
    pub const fn new(bump: u8, investment: UserInvestment) -> Self {
        Self {
            pda_type: PdaType::IcoInvestment,
            bump,
            investment,
        }
    }

    /// Get the PDA's address.
    ///
    /// This function should **not** be used by the On Chain program.
    ///
    /// # Parameters
    /// * `user` - The user owning the investment,
    /// * `program_id` - Program owning the PDA.
    ///
    /// # Returns
    /// * Tuple of public Key of the investment record and associated bump
    #[must_use]
    pub fn get_address(user: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"BangkIcoInvestment", &user.to_bytes()], program_id)
    }
}

impl BangkPda for UserInvestmentPda {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn get_bump(&self) -> u8 {
        self.bump
    }

    fn is_valid(&self) -> bool {
        self.pda_type == PdaType::IcoInvestment
    }

    #[must_use]
    fn seeds(&self) -> Vec<Vec<u8>> {
        vec![
            b"BangkIcoInvestment".to_vec(),
            self.investment.user.to_bytes().to_vec(),
            vec![self.bump],
        ]
    }
}
