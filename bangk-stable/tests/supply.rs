// File: bangk-stable/tests/supply.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 31 December 2024 @ 16:19:35
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

use std::{error, result};

pub mod common;
use bangk_onchain_common::Error as BangkError;
use bangk_stable::get_stable_coin_mint;
use common::{
    burn_coins, close_account, create_coin, get_ata, get_exchange, mint_coins, mint_exchange_coins,
    to_tokens,
};
use solana_program_test::tokio;

const CURRENCY: &str = "Euro BANGK";
const SYMBOL: &str = "EUB";
const URI: &str = "https://api.bangk.app/token-eub";
const DECIMALS: u8 = 2;
const AMOUNT: f64 = 1000.;
const USER: &str = "User 1";

#[tokio::test]
async fn mint_operation() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    mint_coins(&mut env, SYMBOL, USER, AMOUNT).await?;

    let ata = get_ata(&env, USER, SYMBOL);
    let amount = env
        .get_token_amount(&ata)
        .await
        .ok_or("could not retrieve the token amount")?;
    let expected = AMOUNT as u64 * 10_u64.pow(u32::from(DECIMALS));
    assert_eq!(expected, amount);

    Ok(())
}

#[tokio::test]
async fn mint_burn_checks() -> Result<()> {
    let mut env = common::init_default().await?;
    let res1 = mint_coins(&mut env, SYMBOL, USER, AMOUNT).await;
    assert_eq!(res1, Err(BangkError::InvalidPdaAddress));
    let res2 = mint_exchange_coins(&mut env, SYMBOL, AMOUNT).await;
    assert_eq!(res2, Err(BangkError::InvalidPdaAddress));
    let res3 = burn_coins(&mut env, SYMBOL, USER, AMOUNT).await;
    assert_eq!(res3, Err(BangkError::InvalidPdaAddress));

    create_coin(&mut env, CURRENCY, SYMBOL, URI, DECIMALS).await?;
    mint_coins(&mut env, SYMBOL, USER, AMOUNT).await?;

    let res4 = mint_coins(&mut env, SYMBOL, USER, -1.0).await;
    assert_eq!(res4, Err(BangkError::InvalidAmount));
    let res5 = mint_exchange_coins(&mut env, SYMBOL, -1.0).await;
    assert_eq!(res5, Err(BangkError::InvalidAmount));
    let res6 = burn_coins(&mut env, SYMBOL, USER, -1.0).await;
    assert_eq!(res6, Err(BangkError::InvalidAmount));
    let res7 = burn_coins(&mut env, SYMBOL, USER, AMOUNT * 2.0).await;
    assert_eq!(res7, Err(BangkError::InvalidAmount));

    Ok(())
}

#[tokio::test]
async fn mint_operation_existing_ata() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    mint_coins(&mut env, SYMBOL, USER, AMOUNT).await?;
    mint_coins(&mut env, SYMBOL, USER, AMOUNT).await?;

    let ata = get_ata(&env, USER, SYMBOL);
    let amount = env
        .get_token_amount(&ata)
        .await
        .ok_or("could not retrieve the token amount")?;
    let expected = AMOUNT as u64 * 2 * 10_u64.pow(u32::from(DECIMALS));
    assert_eq!(expected, amount);

    Ok(())
}

#[tokio::test]
async fn mint_to_exchange() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    let mint_address = get_stable_coin_mint(SYMBOL);
    let pda_exchange = get_exchange(SYMBOL);
    println!("{mint_address}");
    mint_exchange_coins(&mut env, SYMBOL, AMOUNT).await?;

    // Chechking exchange wallets
    let pda = env.get_account_state(&pda_exchange).await;
    assert_eq!(pda.mint, mint_address);
    let tokens = env.get_token_amount(&pda_exchange).await;
    let expected = AMOUNT as u64 * 10_u64.pow(u32::from(DECIMALS));
    assert_eq!(tokens, Some(expected));

    Ok(())
}

#[tokio::test]
async fn burn_operation() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    mint_coins(&mut env, SYMBOL, USER, AMOUNT * 2.0).await?;
    burn_coins(&mut env, SYMBOL, USER, AMOUNT).await?;

    let ata = get_ata(&env, USER, SYMBOL);
    let amount = env
        .get_token_amount(&ata)
        .await
        .ok_or("could not retrieve the token amount")?;
    let expected = to_tokens(AMOUNT, DECIMALS);
    assert_eq!(expected, amount);

    Ok(())
}

#[tokio::test]
async fn burn_and_close() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    mint_coins(&mut env, SYMBOL, USER, AMOUNT).await?;
    burn_coins(&mut env, SYMBOL, USER, AMOUNT).await?;
    close_account(&mut env, SYMBOL, USER).await?;

    let ata = get_ata(&env, USER, SYMBOL);
    assert!(env.get_account(&ata).await.is_none());

    Ok(())
}

#[tokio::test]
async fn close_non_empty_account() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    mint_coins(&mut env, SYMBOL, USER, AMOUNT).await?;
    let res = close_account(&mut env, SYMBOL, USER).await;

    assert_eq!(
        res,
        Err(BangkError::InvalidAtaData),
        "actual result: {res:#?}"
    );

    Ok(())
}
