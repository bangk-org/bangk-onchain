// File: bangk/src/state/clients.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/clients.rs
// Project: bangk-onchain
// Creation date: Monday 04 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:46:37
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{debug, get_timestamp, pda::PdaType, Error};
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::utils::to_key;

use super::pda::BangkPda;

/// Record for a client's investment
#[derive(BorshSerialize, BorshDeserialize, Debug, ShankAccount)]
pub struct Investment<'a> {
    /// PDA Type (Must always be [`PdaType::UserProjectInvestment`]),
    pub pda_type: PdaType,
    /// Seed bump to access the PDA on the chain.
    pub bump: u8,
    /// ID of the client owning the investment.
    pub client: String,
    /// Mint of the project this record is for.
    pub project: String,
    /// ATA used by the client.
    pub ata: String,
    /// Creation date of the record (and investment).
    pub creation: i64,
    /// Date of the last interest payment.
    pub last_payment: i64,
    /// Account where the PDA is stored on the chain
    #[borsh(skip)]
    pub account: Option<AccountInfo<'a>>,
}

impl<'a> Investment<'a> {
    /// Creates a new client investment.
    ///
    /// # Parameters
    /// * `client` - Public key of the investing client,
    /// * `project` - Public key on the mint of the project the client invests to,
    /// * `currency` - Mint of the stable coin the client is investing with.
    #[must_use]
    pub fn new(
        account: AccountInfo<'a>,
        client: &Pubkey,
        project: &Pubkey,
        ata: &Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            pda_type: PdaType::UserProjectInvestment,
            bump,
            client: client.to_string(),
            project: project.to_string(),
            ata: ata.to_string(),
            creation: get_timestamp().unwrap_or_default(),
            last_payment: 0,
            account: Some(account),
        }
    }
}

impl<'a> BangkPda<'a> for Investment<'a> {
    fn name() -> &'static str {
        "InvestmentRecord"
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }

    fn is_valid(&self) -> bool {
        self.pda_type == PdaType::UserProjectInvestment
    }

    fn set_account(&mut self, account: AccountInfo<'a>) {
        self.account = Some(account);
    }

    fn get_account(&self) -> Result<AccountInfo<'a>, ProgramError> {
        Ok(self.account.clone().ok_or(Error::MissingPDAAccount)?)
    }

    #[must_use]
    fn seeds(&self) -> Vec<Vec<u8>> {
        let client_key = to_key(&self.client).unwrap_or_default();
        let project_key = to_key(&self.project).unwrap_or_default();
        debug!(
            "client_key: {:?}, project_key: {:?}",
            client_key, project_key
        );
        Self::_seeds(&[&client_key, &project_key])
    }
}
