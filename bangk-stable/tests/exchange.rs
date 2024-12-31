// File: bangk-stable/tests/exchange.rs
// Project: bangk-onchain
// Creation date: Monday 23 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 31 December 2024 @ 16:40:55
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic_in_result_fn)]
#![allow(clippy::print_stdout)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{collections::HashMap, error, result};

use bangk_onchain_common::Error as BangkError;
use bangk_stable::{get_stable_coin_mint, update_exchange_rates, ConfigurationPda};
use common::{
    add_freeze_key, create_coin, exchange_coins, freeze_ata, get_ata, get_exchange, mint_coins,
    mint_exchange_coins, thaw_ata, to_tokens,
};
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};
use tests_utilities::onchain::Environment;
pub mod common;

const FREEZE_USER1: &str = "Freeze 1";
const FREEZE_USER2: &str = "Freeze 2";

const CURRENCY_EUR: &str = "Euro BANGK";
const SYMBOL_EUR: &str = "EUB";
const CURRENCY_JPY: &str = "Yen BANGK";
const SYMBOL_JPY: &str = "JPB";
const CURRENCY_USD: &str = "Dollar BANGK";
const SYMBOL_USD: &str = "USD";
const URI: &str = "https://api.bangk.app/token-token";
const DECIMALS_JPY: u8 = 0;
const DECIMALS_DEFAULT: u8 = 2;

const AMOUNT: f64 = 1000.;
const AMOUNT_EXCHANGE_0D: f64 = 1_000_000.0;
const AMOUNT_EXCHANGE_2D: f64 = 100_000.0;

const RATE_EUR_JPY: f64 = 150.0;
const RATE_EUR_USD: f64 = 1.1;

const USER_SOURCE: &str = "User 1";
const USER_TARGET: &str = "User 2";

fn get_default_rates() -> HashMap<Pubkey, f64> {
    HashMap::from([
        (get_stable_coin_mint(SYMBOL_JPY), RATE_EUR_JPY),
        (get_stable_coin_mint(SYMBOL_USD), RATE_EUR_USD),
    ])
}

async fn setup() -> Result<Environment> {
    let mut env = common::init_default().await?;

    // Set exchange rates
    let rates = get_default_rates();

    let admin1 = env.wallets["Admin 1"].pubkey();
    let instruction = update_exchange_rates(&admin1, rates);
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1"])
        .await?;

    // At this point exchange rates are set correctly
    create_coin(&mut env, CURRENCY_EUR, SYMBOL_EUR, URI, 2).await?;
    mint_exchange_coins(&mut env, SYMBOL_EUR, AMOUNT_EXCHANGE_2D).await?;

    create_coin(&mut env, CURRENCY_JPY, SYMBOL_JPY, URI, DECIMALS_JPY).await?;
    mint_exchange_coins(&mut env, SYMBOL_JPY, AMOUNT_EXCHANGE_0D).await?;
    create_coin(&mut env, CURRENCY_USD, SYMBOL_USD, URI, DECIMALS_DEFAULT).await?;
    mint_exchange_coins(&mut env, SYMBOL_USD, AMOUNT_EXCHANGE_2D).await?;

    Ok(env)
}

#[tokio::test]
async fn set_exchange_rates() -> Result<()> {
    let mut env = common::init_default().await?;

    // Testing configuration PDA integrity
    let (config_pda, _) = ConfigurationPda::get_address(&bangk_stable::ID);
    let config_before: ConfigurationPda = env
        .from_account(&config_pda)
        .await
        .ok_or("could not load the ICO program configuration")?;

    assert!(config_before.exchange_rates.is_empty());

    // Set exchange rates
    let rates = get_default_rates();

    let admin1 = env.wallets["Admin 1"].pubkey();
    let instruction = update_exchange_rates(&admin1, rates);
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1"])
        .await?;

    let config_after: ConfigurationPda = env
        .from_account(&config_pda)
        .await
        .ok_or("could not load the ICO program configuration")?;

    assert_eq!(config_after.exchange_rates.len(), 2);
    assert_eq!(
        config_after
            .exchange_rates
            .get(&get_stable_coin_mint(SYMBOL_JPY))
            .copied(),
        Some(RATE_EUR_JPY)
    );
    assert_eq!(
        config_after
            .exchange_rates
            .get(&get_stable_coin_mint(SYMBOL_USD))
            .copied(),
        Some(RATE_EUR_USD)
    );

    Ok(())
}

#[tokio::test]
async fn set_invalid_rates() -> Result<()> {
    let mut env = common::init_default().await?;
    let admin1 = env.wallets["Admin 1"].pubkey();

    let rates1 = HashMap::from([
        (get_stable_coin_mint(SYMBOL_JPY), -3.0_f64),
        (get_stable_coin_mint(SYMBOL_USD), -2.0_f64),
    ]);
    let instruction1 = update_exchange_rates(&admin1, rates1);
    let res1 = env.execute_transaction(&[instruction1], &["Admin 1"]).await;
    assert_eq!(res1, Err(BangkError::InvalidExchangeRate));

    let rates2 = HashMap::from([
        (get_stable_coin_mint(SYMBOL_JPY), -0.0_f64),
        (get_stable_coin_mint(SYMBOL_USD), 1.1_f64),
    ]);
    let instruction2 = update_exchange_rates(&admin1, rates2);
    let res2 = env.execute_transaction(&[instruction2], &["Admin 1"]).await;
    assert_eq!(res2, Err(BangkError::InvalidExchangeRate));

    Ok(())
}

#[tokio::test]
async fn check_setup() -> Result<()> {
    let mut env = setup().await?;

    let exchange_source = get_exchange(SYMBOL_JPY);
    let exchange_target = get_exchange(SYMBOL_USD);

    let expected_source = AMOUNT_EXCHANGE_0D as u64 * 10_u64.pow(u32::from(DECIMALS_JPY));
    let expected_target = AMOUNT_EXCHANGE_2D as u64 * 10_u64.pow(u32::from(DECIMALS_DEFAULT));
    let tokens_source = env.get_token_amount(&exchange_source).await;
    let tokens_target = env.get_token_amount(&exchange_target).await;
    assert_eq!(tokens_source, Some(expected_source));
    assert_eq!(tokens_target, Some(expected_target));

    Ok(())
}

#[tokio::test]
async fn exchange_eur_foreign() -> Result<()> {
    let mut env = setup().await?;

    let expected_cost = AMOUNT / RATE_EUR_JPY;

    mint_coins(&mut env, SYMBOL_EUR, USER_SOURCE, expected_cost * 2.0).await?;
    mint_coins(&mut env, SYMBOL_JPY, USER_TARGET, 0.0).await?;
    exchange_coins(
        &mut env,
        SYMBOL_EUR,
        SYMBOL_JPY,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await?;

    let exchange_source = get_exchange(SYMBOL_EUR);
    let exchange_target = get_exchange(SYMBOL_JPY);

    assert_eq!(
        env.get_token_amount(&exchange_source).await,
        Some(to_tokens(
            AMOUNT_EXCHANGE_2D + AMOUNT / RATE_EUR_JPY,
            DECIMALS_DEFAULT
        ))
    );
    assert_eq!(
        env.get_token_amount(&exchange_target).await,
        Some(to_tokens(AMOUNT_EXCHANGE_0D - AMOUNT, DECIMALS_JPY))
    );

    let source_ata = get_ata(&env, USER_SOURCE, SYMBOL_EUR);
    let target_ata = get_ata(&env, USER_TARGET, SYMBOL_JPY);

    println!("Checking that the users have the expected amount of tokens");
    assert_eq!(
        env.get_token_amount(&source_ata).await,
        Some(to_tokens(expected_cost.mul_add(2.0, -(AMOUNT / RATE_EUR_JPY)), 2) - 1)
    );
    assert_eq!(
        env.get_token_amount(&target_ata).await,
        Some(to_tokens(AMOUNT, DECIMALS_JPY))
    );

    Ok(())
}

#[tokio::test]
async fn exchange_foreign_eur() -> Result<()> {
    let mut env = setup().await?;

    let expected_cost = AMOUNT * RATE_EUR_JPY;

    mint_coins(&mut env, SYMBOL_JPY, USER_SOURCE, expected_cost * 2.0).await?;
    mint_coins(&mut env, SYMBOL_EUR, USER_TARGET, 0.0).await?;
    exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_EUR,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await?;

    let exchange_source = get_exchange(SYMBOL_JPY);
    let exchange_target = get_exchange(SYMBOL_EUR);

    assert_eq!(
        env.get_token_amount(&exchange_source).await,
        Some(to_tokens(
            AMOUNT.mul_add(RATE_EUR_JPY, AMOUNT_EXCHANGE_0D),
            DECIMALS_JPY
        ))
    );
    assert_eq!(
        env.get_token_amount(&exchange_target).await,
        Some(to_tokens(AMOUNT_EXCHANGE_2D - AMOUNT, DECIMALS_DEFAULT))
    );

    let source_ata = get_ata(&env, USER_SOURCE, SYMBOL_JPY);
    let target_ata = get_ata(&env, USER_TARGET, SYMBOL_EUR);

    println!("Checking that the users have the expected amount of tokens");
    assert_eq!(
        env.get_token_amount(&source_ata).await,
        Some(to_tokens(
            expected_cost.mul_add(2.0, -(AMOUNT * RATE_EUR_JPY)),
            DECIMALS_JPY
        ))
    );
    assert_eq!(
        env.get_token_amount(&target_ata).await,
        Some(to_tokens(AMOUNT, DECIMALS_DEFAULT))
    );

    Ok(())
}

#[tokio::test]
async fn exchange_foreign_foreign() -> Result<()> {
    let mut env = setup().await?;
    let rate = 1.0_f64 / RATE_EUR_JPY * RATE_EUR_USD;
    println!("rate is {rate}");

    let expected_cost = AMOUNT / rate;

    mint_coins(&mut env, SYMBOL_JPY, USER_SOURCE, expected_cost * 2.0).await?;
    mint_coins(&mut env, SYMBOL_USD, USER_TARGET, 0.0).await?;
    exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_USD,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await?;

    let exchange_source = get_exchange(SYMBOL_JPY);
    let exchange_target = get_exchange(SYMBOL_USD);

    assert_eq!(
        env.get_token_amount(&exchange_target).await,
        Some(to_tokens(AMOUNT_EXCHANGE_2D - AMOUNT, DECIMALS_DEFAULT))
    );
    assert_eq!(
        env.get_token_amount(&exchange_source).await,
        Some(to_tokens(
            (AMOUNT_EXCHANGE_0D + AMOUNT / rate).ceil(),
            DECIMALS_JPY
        ))
    );

    let source_ata = get_ata(&env, USER_SOURCE, SYMBOL_JPY);
    let target_ata = get_ata(&env, USER_TARGET, SYMBOL_USD);

    println!("Checking that the users have the expected amount of tokens");
    assert_eq!(
        env.get_token_amount(&source_ata).await,
        Some(to_tokens(expected_cost.mul_add(2.0, -(AMOUNT / rate)), DECIMALS_JPY) - 1)
    );
    assert_eq!(
        env.get_token_amount(&target_ata).await,
        Some(to_tokens(AMOUNT, DECIMALS_DEFAULT))
    );

    Ok(())
}

#[tokio::test]
async fn not_enough_exchange_funds() -> Result<()> {
    let mut env = common::init_default().await?;

    // Set exchange rates
    let rates = get_default_rates();

    let admin1 = env.wallets["Admin 1"].pubkey();
    let instruction1 = update_exchange_rates(&admin1, rates);
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction1], &["Admin 1"])
        .await?;

    // At this point exchange rates are set correctly

    create_coin(&mut env, CURRENCY_JPY, SYMBOL_JPY, URI, DECIMALS_JPY).await?;
    mint_exchange_coins(&mut env, SYMBOL_JPY, AMOUNT_EXCHANGE_0D).await?;
    create_coin(&mut env, CURRENCY_USD, SYMBOL_USD, URI, DECIMALS_DEFAULT).await?;

    let rate = 1.0_f64 / RATE_EUR_JPY * RATE_EUR_USD;
    println!("rate is {rate}");

    let expected_cost = AMOUNT / rate;

    mint_coins(&mut env, SYMBOL_JPY, USER_SOURCE, expected_cost * 2.0).await?;
    mint_coins(&mut env, SYMBOL_USD, USER_TARGET, 0.0).await?;

    let res = exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_USD,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await;
    assert_eq!(
        res,
        Err(BangkError::InsufficientExchangeFunds),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn invalid_amount() -> Result<()> {
    let mut env = setup().await?;

    mint_coins(&mut env, SYMBOL_JPY, USER_SOURCE, AMOUNT).await?;
    mint_coins(&mut env, SYMBOL_USD, USER_TARGET, 0.0).await?;

    let res1 = exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_USD,
        USER_SOURCE,
        USER_TARGET,
        -1.0,
    )
    .await;
    assert_eq!(res1, Err(BangkError::InvalidAmount));

    let res2 = exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_USD,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await;
    assert_eq!(res2, Err(BangkError::InvalidAmount));

    Ok(())
}

#[tokio::test]
async fn exchange_with_frozen_account() -> Result<()> {
    let mut env = setup().await?;
    add_freeze_key(&mut env, FREEZE_USER1).await?;
    add_freeze_key(&mut env, FREEZE_USER2).await?;

    let rate = 1.0_f64 / RATE_EUR_JPY * RATE_EUR_USD;
    let expected_cost = AMOUNT / rate;
    mint_coins(&mut env, SYMBOL_JPY, USER_SOURCE, expected_cost * 2.0).await?;
    mint_coins(&mut env, SYMBOL_USD, USER_TARGET, 0.0).await?;

    freeze_ata(&mut env, USER_SOURCE, SYMBOL_JPY).await?;
    let res1 = exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_USD,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await;
    assert_eq!(res1, Err(BangkError::InvalidFreezeStatus));
    thaw_ata(&mut env, USER_SOURCE, SYMBOL_JPY).await?;

    freeze_ata(&mut env, USER_TARGET, SYMBOL_USD).await?;
    let res2 = exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_USD,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await;
    assert_eq!(res2, Err(BangkError::InvalidFreezeStatus));
    thaw_ata(&mut env, USER_TARGET, SYMBOL_USD).await?;

    mint_coins(&mut env, SYMBOL_USD, USER_SOURCE, AMOUNT).await?;
    freeze_ata(&mut env, USER_SOURCE, SYMBOL_USD).await?;
    exchange_coins(
        &mut env,
        SYMBOL_JPY,
        SYMBOL_USD,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await?;

    Ok(())
}
