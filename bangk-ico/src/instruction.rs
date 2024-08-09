// File: bangk-ico/src/instruction.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 25 July 2024 @ 21:36:42
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
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::timelock::TransferFromReserveTimelock;
use crate::{
    config::ConfigurationPda,
    investment::UserInvestmentPda,
    unvesting::{UnvestingScheme, UnvestingType},
};

/// Arguments for the program's initialization.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct InitializeArgs {
    /// Definition of the unvesting schemes.
    pub unvesting: Vec<UnvestingScheme>,
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

/// Arguments for BGK Mint creation and initial minting.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct MintCreationArgs {
    /// Seed bump for the Mint's account
    pub bump: u8,
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

/// Arguments to create / update a user's investment.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct UserInvestmentArgs {
    /// PDA bump.
    pub bump: u8,
    /// User owning the investment
    pub user: Pubkey,
    /// Type of investment.
    pub invest_kind: UnvestingType,
    /// Custom rule of unvesting.
    pub custom_rule: Option<UnvestingScheme>,
    /// Amount of tokens
    pub amount: u64,
}

/// Arguments to delete a user's investment.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct CancelInvestmentArgs {
    /// User owning the investment
    pub user: Pubkey,
    /// Investment type to cancel
    pub kind: UnvestingType,
    /// Invested amount to cancel
    pub amount: u64,
}

/// Arguments to set the BGK launch date.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct LaunchBGKArgs {
    /// Timestamp of the launch date.
    pub timestamp: i64,
    /// Amount of tokens that will be unvested.
    pub amount: u64,
}

/// Transfer BGK out of Bangk's reserve account.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct QueueTransferFromReserveArgs {
    /// ATA to which the tokens will be transferred.
    pub target: Pubkey,
    /// Amount of tokens to transfer.
    pub amount: u64,
}

/// Transfer BGK out of Bangk's reserve account.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct ExecuteTransferFromReserveArgs {
    /// Amount of tokens to transfer.
    pub amount: u64,
}

/// Global payload for Bangk program.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, ShankInstruction)]
#[rustfmt::skip]
pub enum BangkIcoInstruction {
    /// Initialize the program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(2, writable, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(3, writable, name="transfer_from_reserve_timelock", desc="This PDA will hold timelocked instructions to transfer tokens from the reserve")]
    #[account(4, name="system_program", desc="System Program")]
    Initialize(InitializeArgs),

    /// Create the BGK mint and mint the tokens.
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(4, writable, name="bgk_mint", desc="Mint of the BGK token")]
    #[account(5, writable, name="reserve_ata", desc="ATA Bangk will use as the reserve for BGK tokens")]
    #[account(6, name="system_program", desc="System Program")]
    #[account(7, name="token_program", desc="SPL Token 2022 Program")]
    MintBGK(MintCreationArgs),

    /// Update the keys for the Admin `MultiSig`
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(4, name="system_program", desc="System Program")]
    UpdateAdminMultisig(UpdateAdminMultisigArgs),

    /// Create or update a User's Investment.
    #[account(0, signer, writable, name="payer", desc="Signer and fee payer for the instruction")]
    #[account(1, writable, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(2, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(3, writable, name="user_investment", desc="The PDA in which the details of a user's investment are stored")]
    #[account(4, name="system_program", desc="System Program")]
    UserInvestment(UserInvestmentArgs),

    /// Cancel a user's investment.
    #[account(0, signer, writable, name="admin1", desc="Signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, writable, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(4, writable, name="user_investment", desc="The PDA in which the details of a user's investment are stored")]
    #[account(5, name="system_program", desc="System Program")]
    CancelInvestment(CancelInvestmentArgs),

    /// Set the BGK token launch date.
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, writable, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(4, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(5, name="bgk_mint", desc="Mint of the BGK token")]
    #[account(6, writable, name="reserve_ata", desc="ATA Bangk will use as the reserve for BGK tokens")]
    #[account(7, writable, name="invested_ata", desc="ATA Bangk will use to store BGK tokens that will be gradually released to the users")]
    #[account(8, name="system_program", desc="System Program")]
    #[account(9, name="token_program", desc="SPL Token 2022 Program")]
    #[account(10, name="ata_program", desc="Associated Token Account Program")]
    LaunchBGK(LaunchBGKArgs),

    /// Release tokens (if possible).
    #[account(0, signer, writable, name="payer", desc="Signer and fee payer for the instruction")]
    #[account(1, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(2, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(3, name="bgk_mint", desc="Mint of the BGK token")]
    #[account(4, writable, name="invested_ata", desc="ATA Bangk will use to store BGK tokens that will be gradually released to the users")]
    #[account(5, name="user", desc="Wallet of the user for whom the tokens will be released")]
    #[account(6, writable, name="user_investment", desc="The PDA in which the details of a user's investment are stored")]
    #[account(7, writable, name="user_ata", desc="BGK ATA for the user")]
    #[account(8, name="system_program", desc="System Program")]
    #[account(9, name="token_program", desc="SPL Token 2022 Program")]
    #[account(10, name="ata_program", desc="Associated Token Account Program")]
    VestingRelease,

    /// Queues a transfer request from Bangk's reserve ATA.
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(4, name="timelock", desc="This PDA will hold timelocked instructions to transfer tokens from the reserve")]
    #[account(5, name="system_program", desc="System Program")]
    QueueTransferFromReserve(QueueTransferFromReserveArgs),

    /// Executes a transfer BGK from Bangk's reserve ATA.
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(2, name="timelock", desc="This PDA will hold timelocked instructions to transfer tokens from the reserve")]
    #[account(3, name="bgk_mint", desc="Mint of the BGK token")]
    #[account(4, writable, name="reserve_ata", desc="ATA Bangk will use to store BGK tokens that will be gradually released to the users")]
    #[account(5, name="user", desc="Wallet of the user to whom the tokens will be transfered")]
    #[account(6, writable, name="target_ata", desc="BGK ATA where the tokens will be transfered")]
    #[account(7, name="system_program", desc="System Program")]
    #[account(8, name="token_program", desc="SPL Token 2022 Program")]
    #[account(9, name="ata_program", desc="Associated Token Account Program")]
    ExecuteTransferFromReserve(ExecuteTransferFromReserveArgs),
}

/// Initializes the ICO program's configuration.
///
/// # Parameters
/// * `payer` - Signer & Payer account,
/// * `unvesting` - Definition of the unvesting scheme,
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
    unvesting: Vec<UnvestingScheme>,
    api_key: &Pubkey,
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    admin4: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (transfer_timelock_pda, _timelock_bump) =
        TransferFromReserveTimelock::get_address(&crate::ID);

    let args = InitializeArgs {
        unvesting,
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
            AccountMeta::new(transfer_timelock_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::Initialize(args))?,
    })
}

/// Create the instruction for the creation of the BGK mint and the initial mint of the tokens.
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin2` - Key of the third signer of the instruction,
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn create_mint(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (mint_address, mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &crate::ID);
    let reserve_ata = get_associated_token_address_with_program_id(
        &admin_keys_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new_readonly(*admin2, true),
            AccountMeta::new_readonly(*admin3, true),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(reserve_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::MintBGK(MintCreationArgs {
            bump: mint_bump,
        }))?,
    })
}

/// Create the instruction for the creation of the BGK mint and the initial mint of the tokens.
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin2` - Key of the third signer of the instruction,
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
        data: borsh::to_vec(&BangkIcoInstruction::UpdateAdminMultisig(
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

/// Create an instruction to update or create a user's investment.
///
/// # Parameters
/// * `payer` - Transaction signer & fee payer,
/// * `user` - User for whom the investment will be created / updated,
/// * `invest_kind` - Type of investment (private sell, public sells, etc.),
/// * `custom_rule` - Custom rule of unvesting if necessary for Advisers and Partners,
/// * `amount` - Number of tokens bought.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn user_investment(
    payer: &Pubkey,
    user: &Pubkey,
    invest_kind: UnvestingType,
    custom_rule: Option<UnvestingScheme>,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (investment_pda, investment_bump) = UserInvestmentPda::get_address(user, &crate::ID);
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(investment_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::UserInvestment(UserInvestmentArgs {
            bump: investment_bump,
            user: *user,
            invest_kind,
            custom_rule,
            amount,
        }))?,
    })
}

/// Cancel a user's investment.
///
/// # Parameters
/// * `payer` - Transaction signer & fee payer,
/// * `admin` - Admin key signing the instruction,
/// * `user` - User for whom the investment will be created / updated,
/// * `kind` - Investment type to cancel,
/// * `amount` - Amount to cancel.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn cancel_investment(
    payer: &Pubkey,
    admin: &Pubkey,
    user: &Pubkey,
    kind: UnvestingType,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &crate::ID);
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(investment_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::CancelInvestment(
            CancelInvestmentArgs {
                user: *user,
                kind,
                amount,
            },
        ))?,
    })
}

/// Create the instruction to set the BGK token launch date.
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin2` - Key of the third signer of the instruction,
/// * `timestamp` - Timestamp of the launch,
/// * `amount` - Number of tokens to be released during the unvesting.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn launch_bgk(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    timestamp: i64,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &crate::ID);
    let reserve_ata = get_associated_token_address_with_program_id(
        &admin_keys_pda,
        &mint_address,
        &spl_token_2022::ID,
    );
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new_readonly(*admin2, true),
            AccountMeta::new_readonly(*admin3, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(reserve_ata, false),
            AccountMeta::new(invested_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::LaunchBGK(LaunchBGKArgs {
            timestamp,
            amount,
        }))?,
    })
}

/// Create the instruction to release vested tokens.
///
/// # Parameters
/// * `payer` - Wallet signing and paying the transaction.
/// * `user` - User for whom the tokens will be released.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn vesting_release(payer: &Pubkey, user: &Pubkey) -> Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &crate::ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &crate::ID);
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );
    let user_ata =
        get_associated_token_address_with_program_id(user, &mint_address, &spl_token_2022::ID);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new_readonly(mint_address, false),
            AccountMeta::new(invested_ata, false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new(investment_pda, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::VestingRelease)?,
    })
}

/// Queues an instruction to transfer tokens from the reserve.
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin2` - Key of the third signer of the instruction,
/// * `target` - Target ATA (created if doesn't exist yet),
/// * `amount` - Number of tokens to transfer.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn queue_transfer_from_reserve(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    target: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &crate::ID);
    let (transfer_timelock_pda, _timelock_bump) =
        TransferFromReserveTimelock::get_address(&crate::ID);
    let target_ata =
        get_associated_token_address_with_program_id(target, &mint_address, &spl_token_2022::ID);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new_readonly(*admin2, true),
            AccountMeta::new_readonly(*admin3, true),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(transfer_timelock_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::QueueTransferFromReserve(
            QueueTransferFromReserveArgs {
                target: target_ata,
                amount,
            },
        ))?,
    })
}

/// Create the instruction to execute a time-locked transfer
///
/// # Parameters
/// * `target` - Target ATA (created if doesn't exist yet),
/// * `amount` - Number of tokens to be released during the unvesting.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn execute_transfer_from_reserve(
    payer: &Pubkey,
    target: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &crate::ID);
    let (transfer_timelock_pda, _timelock_bump) =
        TransferFromReserveTimelock::get_address(&crate::ID);
    let reserve_ata = get_associated_token_address_with_program_id(
        &admin_keys_pda,
        &mint_address,
        &spl_token_2022::ID,
    );
    let target_ata =
        get_associated_token_address_with_program_id(target, &mint_address, &spl_token_2022::ID);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(transfer_timelock_pda, false),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(reserve_ata, false),
            AccountMeta::new_readonly(*target, false),
            AccountMeta::new(target_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::ExecuteTransferFromReserve(
            ExecuteTransferFromReserveArgs { amount },
        ))?,
    })
}
