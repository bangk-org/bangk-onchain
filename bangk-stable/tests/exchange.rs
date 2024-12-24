// File: bangk-stable/tests/exchange.rs
// Project: bangk-onchain
// Creation date: Monday 23 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 24 December 2024 @ 18:55:36
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
use bangk_stable::{update_exchange_rates, ConfigurationPda};
use common::{
    create_coin, exchange_coins, get_ata, get_exchange, mint_coins, mint_exchange_coins, to_tokens,
};
use solana_sdk::signer::Signer as _;
use tests_utilities::onchain::Environment;
pub mod common;

const SOURCE_CURRENCY: &str = "Yen BANGK";
const SOURCE_SYMBOL: &str = "JPB";
const TARGET_CURRENCY: &str = "Dollar BANGK";
const TARGET_SYMBOL: &str = "USD";
const URI: &str = "https://api.bangk.app/token-token";
const SOURCE_DECIMALS: u8 = 0;
const TARGET_DECIMALS: u8 = 2;
const AMOUNT: f64 = 1000.;

const EXCHANGE_SOURCE_EUB: f64 = 150.0;
const EXCHANGE_SOURCE_AMOUNT: f64 = 1_000_000.0;
const EXCHANGE_TARGET_EUB: f64 = 1.1;
const EXCHANGE_TARGET_AMOUNT: f64 = 100_000.0;

const USER_SOURCE: &str = "User 1";
const USER_TARGET: &str = "User 2";

async fn setup() -> Result<Environment> {
    let mut env = common::init_default().await?;

    // Set exchange rates
    let rates = HashMap::from([
        (SOURCE_SYMBOL.to_owned(), EXCHANGE_SOURCE_EUB),
        (TARGET_SYMBOL.to_owned(), EXCHANGE_TARGET_EUB),
    ]);

    let admin1 = env.wallets["Admin 1"].pubkey();
    let instruction = update_exchange_rates(&admin1, rates)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1"])
        .await?;

    // At this point exchange rates are set correctly

    create_coin(
        &mut env,
        SOURCE_CURRENCY,
        SOURCE_SYMBOL,
        URI,
        SOURCE_DECIMALS,
    )
    .await?;
    mint_exchange_coins(&mut env, SOURCE_SYMBOL, EXCHANGE_SOURCE_AMOUNT).await?;
    create_coin(
        &mut env,
        TARGET_CURRENCY,
        TARGET_SYMBOL,
        URI,
        TARGET_DECIMALS,
    )
    .await?;
    mint_exchange_coins(&mut env, TARGET_SYMBOL, EXCHANGE_TARGET_AMOUNT).await?;

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
    let rates = HashMap::from([
        (SOURCE_SYMBOL.to_owned(), EXCHANGE_SOURCE_EUB),
        (TARGET_SYMBOL.to_owned(), EXCHANGE_TARGET_EUB),
    ]);

    let admin1 = env.wallets["Admin 1"].pubkey();
    let instruction = update_exchange_rates(&admin1, rates)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1"])
        .await?;

    let config_after: ConfigurationPda = env
        .from_account(&config_pda)
        .await
        .ok_or("could not load the ICO program configuration")?;

    assert_eq!(config_after.exchange_rates.len(), 2);
    assert_eq!(
        config_after.exchange_rates.get(SOURCE_SYMBOL).copied(),
        Some(EXCHANGE_SOURCE_EUB)
    );
    assert_eq!(
        config_after.exchange_rates.get(TARGET_SYMBOL).copied(),
        Some(EXCHANGE_TARGET_EUB)
    );

    Ok(())
}

#[tokio::test]
async fn check_setup() -> Result<()> {
    let mut env = setup().await?;

    let exchange_source = get_exchange(SOURCE_SYMBOL);
    let exchange_target = get_exchange(TARGET_SYMBOL);

    let expected_source = EXCHANGE_SOURCE_AMOUNT as u64 * 10_u64.pow(u32::from(SOURCE_DECIMALS));
    let expected_target = EXCHANGE_TARGET_AMOUNT as u64 * 10_u64.pow(u32::from(TARGET_DECIMALS));
    let tokens_source = env.get_token_amount(&exchange_source).await;
    let tokens_target = env.get_token_amount(&exchange_target).await;
    assert_eq!(tokens_source, Some(expected_source));
    assert_eq!(tokens_target, Some(expected_target));

    Ok(())
}

#[tokio::test]
async fn exchange() -> Result<()> {
    let mut env = setup().await?;
    let rate = 1.0_f64 / EXCHANGE_SOURCE_EUB * EXCHANGE_TARGET_EUB;
    println!("rate is {rate}");

    let expected_cost = AMOUNT / rate;

    mint_coins(&mut env, SOURCE_SYMBOL, USER_SOURCE, expected_cost * 2.0).await?;
    mint_coins(&mut env, TARGET_SYMBOL, USER_TARGET, 0.0).await?;
    exchange_coins(
        &mut env,
        SOURCE_SYMBOL,
        TARGET_SYMBOL,
        USER_SOURCE,
        USER_TARGET,
        AMOUNT,
    )
    .await?;

    let exchange_source = get_exchange(SOURCE_SYMBOL);
    let exchange_target = get_exchange(TARGET_SYMBOL);

    assert_eq!(
        env.get_token_amount(&exchange_target).await,
        Some(to_tokens(EXCHANGE_TARGET_AMOUNT - AMOUNT, TARGET_DECIMALS))
    );
    assert_eq!(
        env.get_token_amount(&exchange_source).await,
        Some(to_tokens(
            (EXCHANGE_SOURCE_AMOUNT + AMOUNT / rate).ceil(),
            SOURCE_DECIMALS
        ))
    );

    let source_ata = get_ata(&env, USER_SOURCE, SOURCE_SYMBOL);
    let target_ata = get_ata(&env, USER_TARGET, TARGET_SYMBOL);

    println!("Checking that the users have the expected amount of tokens");
    assert_eq!(
        env.get_token_amount(&source_ata).await,
        Some(to_tokens(
            expected_cost.mul_add(2.0, -(AMOUNT / rate).ceil()),
            SOURCE_DECIMALS
        ))
    );
    assert_eq!(
        env.get_token_amount(&target_ata).await,
        Some(to_tokens(AMOUNT, TARGET_DECIMALS))
    );

    Ok(())
}

#[tokio::test]
async fn not_enough_exchange_funds() -> Result<()> {
    let mut env = common::init_default().await?;

    // Set exchange rates
    let rates = HashMap::from([
        (SOURCE_SYMBOL.to_owned(), EXCHANGE_SOURCE_EUB),
        (TARGET_SYMBOL.to_owned(), EXCHANGE_TARGET_EUB),
    ]);

    let admin1 = env.wallets["Admin 1"].pubkey();
    let instruction1 = update_exchange_rates(&admin1, rates)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction1], &["Admin 1"])
        .await?;

    // At this point exchange rates are set correctly

    create_coin(
        &mut env,
        SOURCE_CURRENCY,
        SOURCE_SYMBOL,
        URI,
        SOURCE_DECIMALS,
    )
    .await?;
    mint_exchange_coins(&mut env, SOURCE_SYMBOL, EXCHANGE_SOURCE_AMOUNT).await?;
    create_coin(
        &mut env,
        TARGET_CURRENCY,
        TARGET_SYMBOL,
        URI,
        TARGET_DECIMALS,
    )
    .await?;

    let rate = 1.0_f64 / EXCHANGE_SOURCE_EUB * EXCHANGE_TARGET_EUB;
    println!("rate is {rate}");

    let expected_cost = AMOUNT / rate;

    mint_coins(&mut env, SOURCE_SYMBOL, USER_SOURCE, expected_cost * 2.0).await?;
    mint_coins(&mut env, TARGET_SYMBOL, USER_TARGET, 0.0).await?;
    let source_key = env.wallets[USER_SOURCE].pubkey();
    let target_key = env.wallets[USER_TARGET].pubkey();
    let instruction2 = bangk_stable::exchange(
        &source_key,
        &target_key,
        SOURCE_SYMBOL,
        TARGET_SYMBOL,
        AMOUNT,
    )?;
    // println!("Instruction: {instruction:#?}");
    let res = env
        .execute_transaction(&[instruction2], &[USER_SOURCE])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::InsufficientExchangeFunds),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}
