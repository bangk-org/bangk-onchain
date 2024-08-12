// File: bangk-ico/src/investment.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 12 August 2024 @ 16:39:22
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_macro::pda;
use bangk_onchain_common::{
    get_timestamp,
    pda::{BangkPda, PdaType},
    Error,
};
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

use crate::unvesting::{UnvestingScheme, UnvestingType};

/// Definition of a user's ICO investment.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq, ShankType)]
pub struct Investment {
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
    pub investments: Vec<Investment>,
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
                    return Err(Error::InvalidUnvestingDefinition.into());
                }
            }
        }
        Ok(Self {
            user,
            investments: vec![Investment {
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
#[pda(kind = PdaType::IcoInvestment, seed = "Investment", seed = investment.user)]
pub struct UserInvestmentPda {
    /// Investment data
    pub investment: UserInvestment,
}

impl<'a> UserInvestmentPda<'a> {
    /// Create a new Account for the definition of Freeze Keys.
    ///
    /// # Parameters
    /// * `bump` - Bump used to derive the PDA address,
    /// * `investment` - Definition of the `UserInvestment`.
    #[must_use]
    pub const fn new(bump: u8, investment: UserInvestment) -> Self {
        Self {
            pda_type: Self::PDA_TYPE,
            bump,
            account: None,
            investment,
        }
    }
}
