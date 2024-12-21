// File: bangk-stable/src/instruction.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:54:24
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::security::{MultiSigPda, MultiSigType};
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;
use solana_program::pubkey::Pubkey;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    system_program,
};

use crate::ConfigurationPda;

/// Arguments for the program's initialization.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct InitializeArgs {
    /// First key in the Admin `MultiSig` (it's the API key)
    pub api_key: Pubkey,
    /// Second key in the Admin `MultiSig`
    pub admin1: Pubkey,
    /// Third key in the Admin `MultiSig`
    pub admin2: Pubkey,
    /// Fourth key in the Admin `MultiSig`
    pub admin3: Pubkey,
    /// Fifth key in the Admin `MultiSig`
    pub admin4: Pubkey,
}

/// Arguments needed to update the admin keys of the program.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct UpdateAdminMultisigArgs {
    /// First key in the Admin `MultiSig` (it's the API key)
    pub api_key: Pubkey,
    /// Second key in the Admin `MultiSig`
    pub admin1: Pubkey,
    /// Third key in the Admin `MultiSig`
    pub admin2: Pubkey,
    /// Fourth key in the Admin `MultiSig`
    pub admin3: Pubkey,
    /// Fifth key in the Admin `MultiSig`
    pub admin4: Pubkey,
}

/// Global payload for Bangk program.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, ShankInstruction)]
#[rustfmt::skip]
pub enum BangkStableInstruction {
    /// Initialize the program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(2, writable, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(3, writable, name="transfer_from_reserve_timelock", desc="This PDA will hold timelocked instructions to transfer tokens from the reserve")]
    #[account(4, name="system_program", desc="System Program")]
    Initialize(InitializeArgs),

    /// Update the keys for the Admin `MultiSig`
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(4, name="system_program", desc="System Program")]
    UpdateAdminMultisig(UpdateAdminMultisigArgs),
}

/// Initializes the ICO program's configuration.
///
/// # Parameters
/// * `payer` - Signer & Payer account,
/// * `api_key` - Key that will initially be used for routine tasks,
/// * `admin1` - First key for the admin `MultiSig`
/// * `admin2` - Second key for the admin `MultiSig`
/// * `admin3` - Third key for the admin `MultiSig`
/// * `admin4` - Fourth key for the admin `MultiSig`
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn initialize(
    payer: &Pubkey,
    api_key: &Pubkey,
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    admin4: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);

    let args = InitializeArgs {
        api_key: *api_key,
        admin1: *admin1,
        admin2: *admin2,
        admin3: *admin3,
        admin4: *admin4,
    };
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new(admin_keys_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::Initialize(args))?,
    })
}

/// Create the instruction for the creation of the BGK mint and the initial mint of the tokens.
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin3` - Key of the third signer of the instruction,
/// * `new_api_key` - Key that will initially be used for routine tasks,
/// * `new_admin1` - First key for the admin `MultiSig`
/// * `new_admin2` - Second key for the admin `MultiSig`
/// * `new_admin3` - Third key for the admin `MultiSig`
/// * `new_admin4` - Fourth key for the admin `MultiSig`
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
#[allow(clippy::too_many_arguments)]
pub fn update_admin_multisig(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    new_api_key: &Pubkey,
    new_admin1: &Pubkey,
    new_admin2: &Pubkey,
    new_admin3: &Pubkey,
    new_admin4: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new_readonly(*admin2, true),
            AccountMeta::new_readonly(*admin3, true),
            AccountMeta::new(admin_keys_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::UpdateAdminMultisig(
            UpdateAdminMultisigArgs {
                api_key: *new_api_key,
                admin1: *new_admin1,
                admin2: *new_admin2,
                admin3: *new_admin3,
                admin4: *new_admin4,
            },
        ))?,
    })
}
