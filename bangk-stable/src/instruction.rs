// File: bangk-stable/src/instruction.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 23 December 2024 @ 20:00:35
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use ::std::hash::BuildHasher;
use std::collections::HashMap;

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

use crate::{ConfigurationPda, EXCHANGE_WALLET_SEED, STABLE_MINT_SEED};

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

/// Arguments needed to create a new Stable Coin mint
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct CreateStableCoinArgs {
    /// Name of the currency' stable coin to initialize
    pub currency: String,
    /// Symbol of the coin.
    pub symbol: String,
    /// `URI` of the coin.
    pub uri: String,
    /// Number of decimals to use.
    pub decimals: u8,
}

/// Arguments needed to update the Name/URI metadata of a Stable Coin
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct UpdateStableCoinMetadataArgs {
    /// Symbol of the coin to update
    pub symbol: String,
    /// New name of the currency' stable coin to initialize
    pub currency: Option<String>,
    /// New `URI` of the coin.
    pub uri: Option<String>,
}

/// Arguments needed to mint Stable Coins.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct StableCoinAmountArgs {
    /// Number of coins to mint/transfer/burn
    pub amount: f64,
}

/// Arguments to set/update the exchange rates between currencies
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct UpdateExchangeRatesArgs {
    /// Map of EUB -> foreign currency rates
    pub exchange_rates: HashMap<String, f64>,
}

/// Arguments needed to exchange two Stable Coins.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct ExchangeArgs {
    /// Source currency
    pub source: String,
    /// Target currency
    pub target: String,
    /// Number of coins to be received
    pub amount: f64,
}

/// Global payload for Bangk program.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, ShankInstruction)]
#[rustfmt::skip]
pub enum BangkStableInstruction {
    /// Initialize the program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(2, writable, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(3, name="system_program", desc="System Program")]
    Initialize(InitializeArgs),

    /// Update the keys for the Admin `MultiSig`
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(4, name="system_program", desc="System Program")]
    UpdateAdminMultisig(UpdateAdminMultisigArgs),

    /// Create a new Stable Coin
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(4, writable, name="mint", desc="Mint of the new stable coin")]
    #[account(5, writable, name="pda_exchange", desc="Bangk wallet for the new coin used in exchange operations")]
    #[account(6, name="system_program", desc="System Program")]
    #[account(7, name="token_program", desc="SPL Token 2022 Program")]
    CreateStableCoin(CreateStableCoinArgs),

    /// Update the Name and/or URI metadata of a stable coin
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(4, writable, name="mint", desc="Mint of the stable coin")]
    #[account(5, name="system_program", desc="System Program")]
    #[account(6, name="token_program", desc="SPL Token 2022 Program")]
    UpdateStableCoinMetadata(UpdateStableCoinMetadataArgs),

    /// Mint Stable Coin to a given user
    #[account(0, signer, writable, name="Bangk", desc="Signer and fee payer for the instruction")]
    #[account(1, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(2, writable, name="mint", desc="Mint of the stable coin")]
    #[account(3, name="user", desc="User receiving the newly minted coins")]
    #[account(4, name="ata", desc="User ATA where the coins are stored")]
    #[account(5, name="system_program", desc="System Program")]
    #[account(6, name="token_program", desc="SPL Token 2022 Program")]
    MintStableCoins(StableCoinAmountArgs),

    /// Mint Stable Coin to the Bangk Exchange PDA
    #[account(0, signer, writable, name="admin1", desc="First signer and fee payer for the instruction")]
    #[account(1, signer, name="admin2", desc="Second signer for the instruction")]
    #[account(2, signer, name="admin3", desc="Third signer for the instruction")]
    #[account(3, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(4, writable, name="mint", desc="Mint of the stable coin")]
    #[account(5, name="exchange_pda", desc="Bangk wallet used in exchange operations")]
    #[account(6, name="system_program", desc="System Program")]
    #[account(7, name="token_program", desc="SPL Token 2022 Program")]
    MintExchangeStableCoins(StableCoinAmountArgs),

    /// Set or update currency exchange rates.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="config_pda", desc="The PDA in which the program's configuration is stored")]
    #[account(2, name="admin_pda", desc="The PDA in which keys allowed to perform administration or routine tasks are stored")]
    #[account(3, name="system_program", desc="System Program")]
    UpdateExchangeRates(UpdateExchangeRatesArgs),

    /// Transfer stable coins from one wallet to another
    #[account(0, signer, writable, name="signer", desc="Wallet owning the tokens to be transfered")]
    #[account(1, name="mint", desc="Mint of the stable coin")]
    #[account(2, writable, name="source_ata", desc="Source ATA of the tokkens to be transfered")]
    #[account(3, writable, name="target_ata", desc="Target ATA receiving the transfered tokkens")]
    #[account(4, name="system_program", desc="System Program")]
    #[account(5, name="token_program", desc="SPL Token 2022 Program")]
    Transfer(StableCoinAmountArgs),

    /// Exchange stable coins from one currency to another
    #[account(0, signer, writable, name="signer", desc="Wallet owning the tokens to be transfered")]
    #[account(1, name="config_pda", desc="The PDA in which the program's configuration is stored (which includes the exchange rates)")]
    #[account(2, name="admin_pda", desc="The PDA in which keys allowed to perform administration tasks are stored")]
    #[account(3, name="source_mint", desc="Mint of the source stable coin")]
    #[account(4, name="target_mint", desc="Mint of the target stable coin")]
    #[account(5, writable, name="source_exchange", desc="Bangk Exchange ATA for the Source currency")]
    #[account(6, writable, name="target_exchange", desc="Bangk Exchange ATA for the target currency")]
    #[account(7, writable, name="source_ata", desc="Source ATA of the tokkens to be exchanged")]
    #[account(8, writable, name="target_ata", desc="Target ATA receiving the exchanged tokkens")]
    #[account(9, name="system_program", desc="System Program")]
    #[account(10, name="token_program", desc="SPL Token 2022 Program")]
    Exchange(ExchangeArgs),
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

/// Create the instruction for the creation of a new stable coin
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin3` - Key of the third signer of the instruction,
/// * `currency` - Name of the Stable Coin to create,
/// * `symbol` - Symbol of the Stable Coin to create,
/// * `uri` - URI of the Stable Coin to create,
/// * `decimals` - Number of decimals used by the Stable Coin.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn create_stable_coin(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    currency: String,
    symbol: String,
    uri: String,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let mint = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), symbol.as_bytes()],
        &crate::ID,
    )
    .0;
    let pda_exchange = Pubkey::find_program_address(
        &[EXCHANGE_WALLET_SEED.as_bytes(), &mint.to_bytes()],
        &crate::ID,
    )
    .0;
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new_readonly(*admin2, true),
            AccountMeta::new_readonly(*admin3, true),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(mint, false),
            AccountMeta::new(pda_exchange, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::CreateStableCoin(
            CreateStableCoinArgs {
                currency,
                symbol,
                uri,
                decimals,
            },
        ))?,
    })
}

/// Create the instruction to update the metadata of a stable coin
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin3` - Key of the third signer of the instruction,
/// * `currency` - New name of the Stable Coin,
/// * `uri` - New URI of the Stable Coin,
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn update_stable_coin(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    symbol: String,
    currency: Option<String>,
    uri: Option<String>,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let mint = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), symbol.as_bytes()],
        &crate::ID,
    )
    .0;
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new_readonly(*admin2, true),
            AccountMeta::new_readonly(*admin3, true),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::UpdateStableCoinMetadata(
            UpdateStableCoinMetadataArgs {
                symbol,
                currency,
                uri,
            },
        ))?,
    })
}

/// Mint some amount of Stable Coin for a given user.
///
/// # Parameters
/// * `admin` - Bangk admin authorizing the operation (should always be the API),
/// * `user` - User receiving the stable coins,
/// * `currency` - Symbol of the Stable Coin,
/// * `amount` - Amount received by the user.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn mint(
    admin: &Pubkey,
    user: &Pubkey,
    currency: &str,
    amount: f64,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let mint = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), currency.as_bytes()],
        &crate::ID,
    )
    .0;
    let ata = get_associated_token_address_with_program_id(user, &mint, &spl_token_2022::id());

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::MintStableCoins(
            StableCoinAmountArgs { amount },
        ))?,
    })
}

/// Mint some amount of Stable Coin for a given user.
///
/// # Parameters
/// * `admin1` - Key of the payer and first signer of the instruction,
/// * `admin2` - Key of the second signer of the instruction,
/// * `admin3` - Key of the third signer of the instruction,
/// * `currency` - Symbol of the Stable Coin,
/// * `amount` - Amount received by the user.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn mint_to_exchange(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    currency: &str,
    amount: f64,
) -> Result<Instruction, ProgramError> {
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let mint = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), currency.as_bytes()],
        &crate::ID,
    )
    .0;
    let pda_exchange = Pubkey::find_program_address(
        &[EXCHANGE_WALLET_SEED.as_bytes(), &mint.to_bytes()],
        &crate::ID,
    )
    .0;

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new(*admin2, true),
            AccountMeta::new(*admin3, true),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(mint, false),
            AccountMeta::new(pda_exchange, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::MintExchangeStableCoins(
            StableCoinAmountArgs { amount },
        ))?,
    })
}

/// Transfer some amount of Stable Coins from one wallet to another.
///
/// # Parameters
/// * `source` - The source wallet sending the coins,
/// * `target` - The target wallet receiving the coins,
/// * `currency` - The currency of the coins to transfer,
/// * `amount` - The number of coins to transfer.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn transfer(
    source: &Pubkey,
    target: &Pubkey,
    currency: &str,
    amount: f64,
) -> Result<Instruction, ProgramError> {
    let mint = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), currency.as_bytes()],
        &crate::ID,
    )
    .0;
    let source_ata =
        get_associated_token_address_with_program_id(source, &mint, &spl_token_2022::id());
    let target_ata =
        get_associated_token_address_with_program_id(target, &mint, &spl_token_2022::id());

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*source, true),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(source_ata, false),
            AccountMeta::new(target_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::Transfer(StableCoinAmountArgs {
            amount,
        }))?,
    })
}

/// Sets or updates the exchange rates between stable coins
///
/// # Parameters
/// * `admin` - Bangk admin authorizing the operation (should always be the API),
/// * `exchange_rates` - The new exchange rates to use.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn update_exchange_rates<S>(
    admin: &Pubkey,
    exchange_rates: HashMap<String, f64, S>,
) -> Result<Instruction, ProgramError>
where
    S: BuildHasher,
    HashMap<String, f64>: From<HashMap<String, f64, S>>,
{
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let exchange_rates = exchange_rates.into();

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::UpdateExchangeRates(
            UpdateExchangeRatesArgs { exchange_rates },
        ))?,
    })
}

/// Transfer some amount of Stable Coins from one wallet to another.
///
/// # Parameters
/// * `source` - The source wallet sending the coins,
/// * `target` - The target wallet receiving the coins,
/// * `currency_source` - The source currency of the coins to exchange,
/// * `currency_target` - The target currency of the coins to exchange,
/// * `amount` - The number of coins to transfer.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn exchange(
    source: &Pubkey,
    target: &Pubkey,
    currency_source: &str,
    currency_target: &str,
    amount: f64,
) -> Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);
    let mint_source = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), currency_source.as_bytes()],
        &crate::ID,
    )
    .0;
    let mint_target = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), currency_target.as_bytes()],
        &crate::ID,
    )
    .0;
    let source_ata =
        get_associated_token_address_with_program_id(source, &mint_source, &spl_token_2022::id());
    let target_ata =
        get_associated_token_address_with_program_id(target, &mint_target, &spl_token_2022::id());
    let exchange_source = Pubkey::find_program_address(
        &[EXCHANGE_WALLET_SEED.as_bytes(), &mint_source.to_bytes()],
        &crate::ID,
    )
    .0;
    let exchange_target = Pubkey::find_program_address(
        &[EXCHANGE_WALLET_SEED.as_bytes(), &mint_target.to_bytes()],
        &crate::ID,
    )
    .0;

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(*source, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new_readonly(mint_source, false),
            AccountMeta::new_readonly(mint_target, false),
            AccountMeta::new(exchange_source, false),
            AccountMeta::new(exchange_target, false),
            AccountMeta::new(source_ata, false),
            AccountMeta::new(target_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
        ],
        data: borsh::to_vec(&BangkStableInstruction::Exchange(ExchangeArgs {
            amount,
            source: currency_source.to_owned(),
            target: currency_target.to_owned(),
        }))?,
    })
}
