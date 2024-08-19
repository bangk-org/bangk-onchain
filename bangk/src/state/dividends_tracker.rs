// File: bangk/src/state/dividends_tracker.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/dividends_tracker.rs
// Project: bangk-onchain
// Creation date: Sunday 25 February 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:46:37
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{debug, pda::PdaType, Error};
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::utils::to_key;

use super::pda::BangkPda;

/// Record for a client's investment
#[derive(BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct DividendsTracker<'a> {
    /// PDA Type (always [`PdaType::ProjectDividendsTracker`]),
    pub pda_type: PdaType,
    /// Seed bump to access the PDA on the chain.
    pub bump: u8,
    /// The project's mint
    pub project: String,
    /// Timestamp of the current or next payment.
    pub payment_date: i64,
    /// Number of clients who are to be paid this round.
    pub total_clients: u32,
    /// Number of clients who have already been paid.
    pub paid_clients: u32,
    /// Account where the PDA is stored on the chain
    #[borsh(skip)]
    pub account: Option<AccountInfo<'a>>,
}

impl<'a> DividendsTracker<'a> {
    /// Create a new Dividends Tracker PDA.
    ///
    /// The record is to be created at the same time
    /// as the project, so all of its fields
    /// are to be zero, except for `pda_type`
    /// which needs to be [`PdaType::ProjectDividendsTracker`].
    #[must_use]
    pub fn new(account: AccountInfo<'a>, project: &Pubkey, bump: u8) -> Self {
        Self {
            pda_type: PdaType::ProjectDividendsTracker,
            bump,
            project: project.to_string(),
            payment_date: 0,
            total_clients: 0,
            paid_clients: 0,
            account: Some(account),
        }
    }
}

impl<'a> BangkPda<'a> for DividendsTracker<'a> {
    fn name() -> &'static str {
        "DevidendsTracker"
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }

    fn is_valid(&self) -> bool {
        self.pda_type == PdaType::ProjectDividendsTracker
    }

    fn set_account(&mut self, account: AccountInfo<'a>) {
        self.account = Some(account);
    }

    fn get_account(&self) -> Result<AccountInfo<'a>, ProgramError> {
        Ok(self.account.clone().ok_or(Error::MissingPDAAccount)?)
    }

    #[must_use]
    fn seeds(&self) -> Vec<Vec<u8>> {
        debug!("as string: {:?}", self.project);
        let project_key = to_key(&self.project).unwrap_or_default();
        debug!("project_key: {:?}", project_key);
        Self::_seeds(&[&project_key])
    }
}
