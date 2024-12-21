// File: bangk-stable/src/processor/main.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:54:24
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use borsh::BorshDeserialize as _;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::BangkStableInstruction;

use super::admin::{initialize, update_admin_multisig};

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
            update_admin_multisig(program_id, accounts, args)
        }
    }
}
