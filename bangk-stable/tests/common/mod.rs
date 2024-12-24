// File: bangk-stable/tests/common/mod.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 24 December 2024 @ 19:02:48
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

use bangk_stable::{
    add_freeze_authority, burn, create_stable_coin, exchange, freeze_account, initialize, mint,
    mint_to_exchange, process_instruction, remove_freeze_authority, thaw_account, transfer,
    EXCHANGE_WALLET_SEED, STABLE_MINT_SEED,
};
use solana_program_test::processor;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use tests_utilities::onchain::Environment;

pub const PROGRAM_ID: Pubkey =
    solana_program::pubkey!("BKPrg5rXBCXMEJPnL2K8DaFEcua1e1SkeLUGqSQhkj6U");

#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn to_tokens(amount: f64, decimals: u8) -> u64 {
    let res = amount * 10_f64.powi(i32::from(decimals));
    res as u64
}

/// Get the address of the mint for a given currency
///
/// # Parameters
/// * `currency_symbol` - The stable coin for which to get the address of the mint
#[must_use]
pub fn get_mint(currency_symbol: &str) -> Pubkey {
    Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), currency_symbol.as_bytes()],
        &bangk_stable::ID,
    )
    .0
}

/// Get the address of the Exchange PDA for a given currency
///
/// # Parameters
/// * `currency_symbol` - The stable coin for which to get the exchange PDA
#[must_use]
pub fn get_exchange(currency_symbol: &str) -> Pubkey {
    Pubkey::find_program_address(
        &[
            EXCHANGE_WALLET_SEED.as_bytes(),
            &get_mint(currency_symbol).to_bytes(),
        ],
        &bangk_stable::ID,
    )
    .0
}

/// Get the ATA for a given couple user & currency
///
/// # Parameters
/// * `env` - The testing environment,
/// * `user` - The user owning the ATA,
/// * `currency_symbol` - The stable coin currency owned by the ATA.
#[must_use]
pub fn get_ata(env: &Environment, user: &str, currency_symbol: &str) -> Pubkey {
    let user = env.wallets[user].pubkey();
    let mint = get_mint(currency_symbol);
    get_associated_token_address_with_program_id(&user, &mint, &spl_token_2022::id())
}

/// Default initialization of the ICO program
///
/// # Errors
/// If the initialization failed
pub async fn init_default() -> Result<Environment> {
    let mut env =
        Environment::new(PROGRAM_ID, "bangk_stable", processor!(process_instruction)).await;
    let api_key = env
        .wallets
        .get("API")
        .ok_or("no API key in the environment")?;
    let api_pub = api_key.pubkey();

    let admin1 = env.add_wallet("Admin 1").await;
    let admin2 = env.add_wallet("Admin 2").await;
    let admin3 = env.add_wallet("Admin 3").await;
    let admin4 = env.add_wallet("Admin 4").await;
    let _user1 = env.add_wallet("User 1").await;
    let _user2 = env.add_wallet("User 2").await;
    let _freeze1 = env.add_wallet("Freeze 1").await;
    let _freeze2 = env.add_wallet("Freeze 2").await;

    let instruction = initialize(&api_pub, &api_pub, &admin1, &admin2, &admin3, &admin4)?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    Ok(env)
}

/// Initializes the testing environment with the mint created and the tokens minted
///
/// # Errors
/// If the initialization failed
pub async fn init_with_mint(
    currency: &str,
    symbol: &str,
    uri: &str,
    decimals: u8,
) -> Result<Environment> {
    let mut env = init_default().await?;
    create_coin(&mut env, currency, symbol, uri, decimals).await?;

    Ok(env)
}

/// Get the exchange PDA address for a given currency
///
/// # Parameters
/// * `symbol` - Currency for which to get the exchange PDA address.
#[must_use]
pub fn get_exchange_pda(symbol: &str) -> Pubkey {
    let mint = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), symbol.as_bytes()],
        &bangk_stable::id(),
    )
    .0;
    Pubkey::find_program_address(
        &[EXCHANGE_WALLET_SEED.as_bytes(), &mint.to_bytes()],
        &bangk_stable::id(),
    )
    .0
}

/// Create a new coin *without* creating the corresponding exchange ATA
///
/// # Parameters
/// * `env` - The testing environment,
/// * `currency` - The currency to mint,
/// * `target` - The user receiving the minted tokens
/// * `amount` - The amount to mint.
///
/// # Errors
/// If the coin could not be created
pub async fn create_coin(
    env: &mut Environment,
    currency: &str,
    symbol: &str,
    uri: &str,
    decimals: u8,
) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let instruction1 = create_stable_coin(
        &admin1,
        &admin2,
        &admin3,
        currency.to_owned(),
        symbol.to_owned(),
        uri.to_owned(),
        decimals,
    )?;
    env.execute_transaction(&[instruction1], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    Ok(())
}

/// Mint tokens
///
/// # Parameters
/// * `env` - The testing environment,
/// * `currency` - The currency to mint,
/// * `target` - The user receiving the minted tokens
/// * `amount` - The amount to mint.
///
/// # Errors
/// If the tokens could not be minted
pub async fn mint_coins(
    env: &mut Environment,
    currency: &str,
    target: &str,
    amount: f64,
) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let target = env.wallets[target].pubkey();
    let instruction = mint(&admin1, &target, currency, amount)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1"])
        .await?;

    Ok(())
}

/// Mint tokens to the given currency’s exchange PDA
///
/// # Parameters
/// * `env` - The testing environment,
/// * `currency` - The currency to mint,
/// * `amount` - The amount to mint.
///
/// # Errors
/// If the tokens could not be minted
pub async fn mint_exchange_coins(env: &mut Environment, currency: &str, amount: f64) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let instruction = mint_to_exchange(&admin1, &admin2, &admin3, currency, amount)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    Ok(())
}

/// Burn tokens
///
/// # Parameters
/// * `env` - The testing environment,
/// * `currency` - The currency to mint,
/// * `target` - The user receiving the minted tokens
/// * `amount` - The amount to mint,
/// * `close_empty` - Determines if an empty account should be closed.
///
/// # Errors
/// If the tokens could not be minted
pub async fn burn_coins(
    env: &mut Environment,
    currency: &str,
    user: &str,
    amount: f64,
    close_empty: bool,
) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let user_key = env.wallets[user].pubkey();
    let instruction = burn(&user_key, currency, amount, &admin1, close_empty)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &[user]).await?;

    Ok(())
}

/// Transfer tokens
///
/// # Parameters
/// * `env` - The testing environment,
/// * `currency` - The currency to mint,
/// * `source` - The user sending the tokens,
/// * `target` - The user receiving the tokens
/// * `amount` - The amount to transfer.
///
/// # Errors
/// If the tokens could not be transfered
pub async fn transfer_coins(
    env: &mut Environment,
    currency: &str,
    source: &str,
    target: &str,
    amount: f64,
) -> Result<()> {
    let source_key = env.wallets[source].pubkey();
    let target_key = env.wallets[target].pubkey();
    let instruction = transfer(&source_key, &target_key, currency, amount)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &[source]).await?;

    Ok(())
}

/// Exchange tokens
///
/// # Parameters
/// * `env` - The testing environment,
/// * `source_currency` - The source currency,
/// * `target_currency` - The target currency,
/// * `source` - The user sending the tokens,
/// * `target` - The user receiving the tokens
/// * `amount` - The amount received by the target.
///
/// # Errors
/// If the tokens could not be transfered
pub async fn exchange_coins(
    env: &mut Environment,
    source_currency: &str,
    target_currency: &str,
    source: &str,
    target: &str,
    amount: f64,
) -> Result<()> {
    let source_key = env.wallets[source].pubkey();
    let target_key = env.wallets[target].pubkey();
    let instruction = exchange(
        &source_key,
        &target_key,
        source_currency,
        target_currency,
        amount,
    )?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &[source]).await?;

    Ok(())
}

/// Add a user to the accounts authorized to freeze accounts
///
/// # Parameters
/// * `env` - The testing environment,
/// * `user` - User to authorize.
///
/// # Errors
/// If the user could not be added
pub async fn add_freeze_key(env: &mut Environment, user: &str) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let user = env.wallets[user].pubkey();
    let instruction = add_freeze_authority(&admin1, &admin2, &admin3, &user)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    Ok(())
}

/// Remove a user from the accounts authorized to freeze accounts
///
/// # Parameters
/// * `env` - The testing environment,
/// * `user` - User whose authority to revoke.
///
/// # Errors
/// If the user could not be removed
pub async fn remove_freeze_key(env: &mut Environment, user: &str) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let user = env.wallets[user].pubkey();
    let instruction = remove_freeze_authority(&admin1, &admin2, &admin3, &user)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    Ok(())
}

/// Freeze an account
///
/// # Parameters
/// * `env` - The testing environment,
/// * `user` - User to owning the ATA,
/// * `currency_symbol` - Currency of the ATA to freeze.
///
/// # Errors
/// If the ATA could not be frozen
pub async fn freeze_ata(env: &mut Environment, user: &str, currency_symbol: &str) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let freeze = env.wallets["Freeze 1"].pubkey();
    let mint = get_mint(currency_symbol);
    let ata = get_ata(env, user, currency_symbol);
    let instruction = freeze_account(&admin1, &freeze, &mint, &ata)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1", "Freeze 1"])
        .await?;

    Ok(())
}

/// Thaw an account
///
/// # Parameters
/// * `env` - The testing environment,
/// * `user` - User to owning the ATA,
/// * `currency_symbol` - Currency of the ATA to thaw.
///
/// # Errors
/// If the ATA could not be thawed
pub async fn thaw_ata(env: &mut Environment, user: &str, currency_symbol: &str) -> Result<()> {
    let admin1 = env.wallets["Admin 1"].pubkey();
    let freeze1 = env.wallets["Freeze 1"].pubkey();
    let freeze2 = env.wallets["Freeze 2"].pubkey();
    let mint = get_mint(currency_symbol);
    let ata = get_ata(env, user, currency_symbol);
    let instruction = thaw_account(&admin1, &freeze1, &freeze2, &mint, &ata)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1", "Freeze 1", "Freeze 2"])
        .await?;

    Ok(())
}
