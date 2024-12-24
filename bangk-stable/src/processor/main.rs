// File: bangk-stable/src/processor/main.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 24 December 2024 @ 17:23:10
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use borsh::BorshDeserialize as _;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::BangkStableInstruction;

use super::{
    admin::{initialize, update_admin_multisig},
    coins::{mint_creation, update_metadata},
    supply::{burn_coin, mint_coin, mint_exchange_coin},
    transfers::{exchange, transfer, update_exchange_rates},
};

include!(concat!(env!("OUT_DIR"), "/keys.rs"));

/// Main processor for the program
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let Ok(payload) = BangkStableInstruction::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };
    match payload {
        BangkStableInstruction::Initialize(args) => initialize(program_id, accounts, &args),
        BangkStableInstruction::UpdateAdminMultisig(args) => {
            update_admin_multisig(program_id, accounts, &args)
        }
        BangkStableInstruction::CreateCoin(args) => mint_creation(program_id, accounts, &args),
        BangkStableInstruction::UpdateCoinMetadata(args) => {
            update_metadata(program_id, accounts, &args)
        }
        BangkStableInstruction::Mint(args) => mint_coin(program_id, accounts, args),
        BangkStableInstruction::MintExchange(args) => {
            mint_exchange_coin(program_id, accounts, args)
        }
        BangkStableInstruction::UpdateExchangeRates(args) => {
            update_exchange_rates(program_id, accounts, &args)
        }
        BangkStableInstruction::Transfer(args) => transfer(program_id, accounts, args),
        BangkStableInstruction::Exchange(args) => exchange(program_id, accounts, &args),
        BangkStableInstruction::Burn(args) => burn_coin(program_id, accounts, args),
    }
}
