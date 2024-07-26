// File: bangk/src/state/stable.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/stable.rs
// Project: bangk-onchain
// Creation date: Monday 26 February 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 25 March 2024 @ 16:28:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{account_info::AccountInfo, program_error::ProgramError};
use spl_token_metadata_interface::state::TokenMetadata;

use super::{get_mint_metadata, mint_data::MintData, token_metadata};

/// Data contained in a project account.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Default, ShankAccount)]
pub struct StableMint {
    /// Name of the stable currency.
    pub name: String,
    /// Symbol of the associated token.
    pub symbol: String,
    /// `URI` for the token.
    pub uri: String,
}

impl<'a> MintData<'a> for StableMint {
    fn get_symbol(&self) -> String {
        self.symbol.clone()
    }

    fn base_seed(symbol: &str) -> String {
        format!("Stable{symbol}")
    }

    fn to_metadata(&self, account: &AccountInfo<'a>) -> Result<TokenMetadata, ProgramError> {
        token_metadata(account.key, &self.name, &self.symbol, &self.uri, Vec::new())
    }
}

impl<'a> TryFrom<AccountInfo<'a>> for StableMint {
    type Error = ProgramError;

    fn try_from(value: AccountInfo<'a>) -> Result<Self, Self::Error> {
        let meta = get_mint_metadata(&value)?;
        Ok(Self {
            name: meta.name,
            symbol: meta.symbol,
            uri: meta.uri,
        })
    }
}
