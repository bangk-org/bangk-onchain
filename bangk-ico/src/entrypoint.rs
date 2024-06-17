// File: bangk-ico/src/entrypoint.rs
// Project: bangk-solana
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 20:35:38
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::{check_id, processor::process_instruction as process};

// declare and export the program's entrypoint
entrypoint!(process_instruction);

// program entrypoint's implementation
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        msg!(
            "Invalid program ID: expected {}, got {}",
            crate::ID,
            program_id
        );
        return Err(ProgramError::IncorrectProgramId);
    }
    process(program_id, accounts, instruction_data)
}
