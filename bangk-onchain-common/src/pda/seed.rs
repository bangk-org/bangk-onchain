// File: bangk-onchain-common/src/pda/seed.rs
// Project: bangk-onchain
// Creation date: Thursday 25 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 25 July 2024 @ 20:27:42
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use solana_program::pubkey::Pubkey;

/// A seed used to derive the address of a PDA.
pub struct Seed {
    data: Vec<u8>,
}

impl From<u8> for Seed {
    fn from(value: u8) -> Self {
        Self { data: vec![value] }
    }
}

impl From<&str> for Seed {
    fn from(value: &str) -> Self {
        Self {
            data: value.as_bytes().to_vec(),
        }
    }
}

impl From<String> for Seed {
    fn from(value: String) -> Self {
        Self {
            data: value.into_bytes(),
        }
    }
}

impl From<Pubkey> for Seed {
    fn from(value: Pubkey) -> Self {
        Self {
            data: value.to_bytes().to_vec(),
        }
    }
}

impl From<&Pubkey> for Seed {
    fn from(value: &Pubkey) -> Self {
        Self {
            data: value.to_bytes().to_vec(),
        }
    }
}

impl From<Seed> for Vec<u8> {
    fn from(value: Seed) -> Self {
        value.data
    }
}
