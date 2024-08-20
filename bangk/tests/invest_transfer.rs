// File: tests-onchain-main/tests/invest_transfer.rs
// Project: bangk-onchain
// Creation date: Tuesday 12 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use bangk::state::projects::Periodicity;
use common::commands::{
    add_client, add_exchange, add_project, add_stable_mint, create_client_investment,
    create_client_investment_exchange, launch_project, mint_stable,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;

use crate::common::{
    commands::{create_ata, transfer_invest, transfer_invest_with_exchange},
    environment::Environment,
};

mod common;

const CLIENT1: &str = "Seller";
const CLIENT2: &str = "Default Buyer";
const CLIENT3: &str = "Foreign Buyer";
const CLIENT4: &str = "New Buyer";
const PROJECT: &str = "Test";
const CURRENCY_DEF: &str = "EuroBangk";
const CURRENCY_ALT: &str = "DollarBangk";
const SYMBOL: &str = "TST";
const URI: &str = "https://bangk.app/test";
const INTEREST_RATE: f64 = 7.5;
const TOKEN_VALUE: u32 = 1;
const PERIODICITY: Periodicity = Periodicity::Daily;
const RISK: u8 = 4;
const AMOUNT: u64 = 100;
const COST: u64 = 100;
const TOKEN_COST: u64 = COST * 100;
const EXCHANGE_RATE_DEF_ALT: f64 = 1.111;
const EXCHANGE_RATE_ALT_DEF: f64 = 0.9001;
const STARTING_AMOUNT: u64 = 200_000;

/// Setup the testing environment for the integration tests that follow
/// This one adds clients investement to test cancelling or closing a project.
async fn setup() -> (
    Environment,
    Pubkey,
    (Pubkey, Pubkey),
    (Pubkey, Pubkey, u8),
    (Pubkey, Pubkey, u8),
) {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, CURRENCY_DEF, 2).await;
    add_stable_mint(&mut env, CURRENCY_ALT, 2).await;
    let _ = add_client(&mut env, PROJECT).await;
    let _ = add_client(&mut env, CLIENT1).await;
    let _ = add_client(&mut env, CLIENT2).await;
    let _ = add_client(&mut env, CLIENT3).await;
    let _ = add_client(&mut env, CLIENT4).await;
    let _ = add_client(&mut env, "Bangk").await;
    let _ = mint_stable(&mut env, CLIENT1, CURRENCY_DEF, STARTING_AMOUNT).await;
    let _ = mint_stable(&mut env, CLIENT2, CURRENCY_DEF, STARTING_AMOUNT).await;
    let _ = mint_stable(&mut env, CLIENT3, CURRENCY_ALT, STARTING_AMOUNT).await;
    let _ = mint_stable(&mut env, CLIENT4, CURRENCY_DEF, STARTING_AMOUNT).await;
    let _ = mint_stable(&mut env, CLIENT4, CURRENCY_ALT, STARTING_AMOUNT).await;
    add_exchange(&mut env, CURRENCY_DEF, STARTING_AMOUNT * 2).await;
    add_exchange(&mut env, CURRENCY_ALT, STARTING_AMOUNT * 2).await;

    let mint = add_project(
        &mut env,
        (PROJECT, SYMBOL, URI),
        CURRENCY_DEF,
        INTEREST_RATE,
        TOKEN_VALUE,
        PERIODICITY,
        RISK,
    )
    .await;

    let (client1_token, client1_record, _) =
        create_client_investment(&mut env, SYMBOL, CLIENT1, CURRENCY_DEF, AMOUNT).await;
    let (client2_token, client2_record, client2_bump) =
        create_client_investment(&mut env, SYMBOL, CLIENT2, CURRENCY_DEF, AMOUNT).await;

    let (client3_token, client3_record, client3_bump) = create_client_investment_exchange(
        &mut env,
        SYMBOL,
        CLIENT3,
        CURRENCY_DEF,
        CURRENCY_ALT,
        AMOUNT,
        EXCHANGE_RATE_ALT_DEF,
    )
    .await;

    launch_project(&mut env, SYMBOL).await;

    (
        env,
        mint,
        (client1_token, client1_record),
        (client2_token, client2_record, client2_bump),
        (client3_token, client3_record, client3_bump),
    )
}

#[tokio::test]
async fn default() {
    let (mut env, _, client1, client2, _) = setup().await;
    transfer_invest(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT2,
        AMOUNT / 2,
        TOKEN_COST / 2,
    )
    .await;

    let seller_stable = env.accounts[format!("{} ({})", CLIENT1, CURRENCY_DEF).as_str()].0;
    let buyer_stable = env.accounts[format!("{} ({})", CLIENT2, CURRENCY_DEF).as_str()].0;
    assert_eq!(
        env.get_token_amount(seller_stable)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT - TOKEN_COST + TOKEN_COST / 2
    );
    assert_eq!(
        env.get_token_amount(client1.0).await.unwrap_or_default(),
        AMOUNT - AMOUNT / 2
    );
    assert_eq!(
        env.get_token_amount(buyer_stable).await.unwrap_or_default(),
        STARTING_AMOUNT - TOKEN_COST - TOKEN_COST / 2
    );
    assert_eq!(
        env.get_token_amount(client2.0).await.unwrap_or_default(),
        AMOUNT + AMOUNT / 2
    );
}

#[tokio::test]
async fn sell_max() {
    let (mut env, _, client1, client2, _) = setup().await;
    transfer_invest(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT2,
        AMOUNT,
        TOKEN_COST,
    )
    .await;

    let seller_stable = env.accounts[format!("{} ({})", CLIENT1, CURRENCY_DEF).as_str()].0;
    let buyer_stable = env.accounts[format!("{} ({})", CLIENT2, CURRENCY_DEF).as_str()].0;
    assert_eq!(
        env.get_token_amount(seller_stable)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT
    );
    assert!(env.get_account(client1.0).await.is_none());
    assert!(env.get_account(client1.1).await.is_none());
    assert_eq!(
        env.get_token_amount(buyer_stable).await.unwrap_or_default(),
        STARTING_AMOUNT - TOKEN_COST * 2
    );
    assert_eq!(
        env.get_token_amount(client2.0).await.unwrap_or_default(),
        AMOUNT * 2
    );
}

#[tokio::test]
async fn new_invest() {
    let (mut env, _, client1, _, _) = setup().await;
    let buyer_token = create_ata(&mut env, CLIENT4, SYMBOL, true).await;

    transfer_invest(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT4,
        AMOUNT / 2,
        TOKEN_COST / 2,
    )
    .await;

    let seller_stable = env.accounts[format!("{} ({})", CLIENT1, CURRENCY_DEF).as_str()].0;
    let buyer_stable = env.accounts[format!("{} ({})", CLIENT4, CURRENCY_DEF).as_str()].0;
    assert_eq!(
        env.get_token_amount(seller_stable).await,
        Some(STARTING_AMOUNT - TOKEN_COST + TOKEN_COST / 2)
    );
    assert_eq!(
        env.get_token_amount(client1.0).await,
        Some(AMOUNT - AMOUNT / 2)
    );
    assert_eq!(
        env.get_token_amount(buyer_stable).await,
        Some(STARTING_AMOUNT - TOKEN_COST / 2)
    );
    assert_eq!(env.get_token_amount(buyer_token).await, Some(AMOUNT / 2));
}

#[tokio::test]
#[should_panic]
async fn too_many() {
    let (mut env, _, _, _, _) = setup().await;
    transfer_invest(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT2,
        AMOUNT * 2,
        TOKEN_COST / 2,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn too_expensive() {
    let (mut env, _, _, _, _) = setup().await;
    transfer_invest(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT2,
        AMOUNT / 2,
        TOKEN_COST * 20,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn zero() {
    let (mut env, _, _, _, _) = setup().await;
    transfer_invest(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT2,
        0,
        TOKEN_COST * 20,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn zero_cost() {
    let (mut env, _, _, _, _) = setup().await;
    transfer_invest(&mut env, SYMBOL, CURRENCY_DEF, CLIENT1, CLIENT2, AMOUNT, 0).await;
}

#[tokio::test]
async fn exchange() {
    let (mut env, _, client1, _, client3) = setup().await;
    let seller_stable = env.accounts[format!("{} ({})", CLIENT1, CURRENCY_DEF).as_str()].0;
    let buyer_stable = env.accounts[format!("{} ({})", CLIENT3, CURRENCY_ALT).as_str()].0;
    let exchange_seller = env.accounts[format!("Bangk ({})", CURRENCY_DEF).as_str()].0;
    let exchange_buyer = env.accounts[format!("Bangk ({})", CURRENCY_ALT).as_str()].0;

    let foreign_cost = TOKEN_COST as f64 * EXCHANGE_RATE_DEF_ALT;
    let buyer_token_before = env.get_token_amount(buyer_stable).await.unwrap_or_default();

    transfer_invest_with_exchange(
        &mut env,
        SYMBOL,
        CURRENCY_ALT,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT3,
        AMOUNT / 2,
        TOKEN_COST / 2,
        EXCHANGE_RATE_ALT_DEF,
    )
    .await;

    assert_eq!(
        env.get_token_amount(client1.0).await.unwrap_or_default(),
        AMOUNT - AMOUNT / 2
    );
    assert_eq!(
        env.get_token_amount(client3.0).await.unwrap_or_default(),
        AMOUNT + AMOUNT / 2
    );
    assert_eq!(
        env.get_token_amount(seller_stable)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT - TOKEN_COST / 2,
    );
    assert_eq!(
        env.get_token_amount(buyer_stable).await.unwrap_or_default(),
        buyer_token_before - (foreign_cost * 0.5) as u64
    );
    assert_eq!(
        env.get_token_amount(exchange_seller)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT * 2 - TOKEN_COST - TOKEN_COST / 2
    );
    assert_eq!(
        env.get_token_amount(exchange_buyer)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT * 2 + (foreign_cost * 1.5) as u64
    );
}

#[tokio::test]
async fn exchange_non_existing_dest() {
    let (mut env, _, client1, _, _) = setup().await;
    let seller_stable = env.accounts[format!("{} ({})", CLIENT1, CURRENCY_DEF).as_str()].0;
    let buyer_stable = env.accounts[format!("{} ({})", CLIENT4, CURRENCY_ALT).as_str()].0;
    let exchange_seller = env.accounts[format!("Bangk ({})", CURRENCY_DEF).as_str()].0;
    let exchange_buyer = env.accounts[format!("Bangk ({})", CURRENCY_ALT).as_str()].0;
    let buyer_token = create_ata(&mut env, CLIENT4, SYMBOL, true).await;
    let buyer_token_before = env.get_token_amount(buyer_stable).await.unwrap_or_default();

    let foreign_cost = TOKEN_COST as f64 * EXCHANGE_RATE_DEF_ALT;
    transfer_invest_with_exchange(
        &mut env,
        SYMBOL,
        CURRENCY_ALT,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT4,
        AMOUNT / 2,
        TOKEN_COST / 2,
        EXCHANGE_RATE_ALT_DEF,
    )
    .await;

    assert_eq!(env.get_token_amount(buyer_token).await, Some(AMOUNT / 2));
    assert_eq!(env.get_token_amount(client1.0).await, Some(AMOUNT / 2));
    assert_eq!(
        env.get_token_amount(seller_stable).await,
        Some(STARTING_AMOUNT - TOKEN_COST / 2)
    );
    assert_eq!(
        env.get_token_amount(exchange_seller).await,
        Some(STARTING_AMOUNT * 2 - TOKEN_COST - TOKEN_COST / 2)
    );
    assert_eq!(
        env.get_token_amount(exchange_buyer).await,
        Some(STARTING_AMOUNT * 2 + (foreign_cost * 1.5) as u64)
    );
    assert_eq!(
        env.get_token_amount(buyer_stable).await,
        Some(buyer_token_before - (foreign_cost * 0.5) as u64)
    );
}

#[tokio::test]
async fn exchange_max() {
    let (mut env, _, seller, _, buyer) = setup().await;
    let seller_stable = env.accounts[format!("{} ({})", CLIENT1, CURRENCY_DEF).as_str()].0;
    let buyer_stable = env.accounts[format!("{} ({})", CLIENT3, CURRENCY_ALT).as_str()].0;
    let exchange_seller = env.accounts[format!("Bangk ({})", CURRENCY_DEF).as_str()].0;
    let exchange_buyer = env.accounts[format!("Bangk ({})", CURRENCY_ALT).as_str()].0;

    let foreign_cost = TOKEN_COST as f64 * EXCHANGE_RATE_DEF_ALT;
    let buyer_token_before = env.get_token_amount(buyer_stable).await.unwrap_or_default();

    transfer_invest_with_exchange(
        &mut env,
        SYMBOL,
        CURRENCY_ALT,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT3,
        AMOUNT,
        TOKEN_COST,
        EXCHANGE_RATE_ALT_DEF,
    )
    .await;

    assert_eq!(env.get_token_amount(seller.0).await.unwrap_or_default(), 0);
    assert_eq!(
        env.get_token_amount(buyer.0).await.unwrap_or_default(),
        AMOUNT * 2
    );
    assert_eq!(
        env.get_token_amount(seller_stable)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT,
    );
    assert_eq!(
        env.get_token_amount(buyer_stable).await.unwrap_or_default(),
        buyer_token_before - foreign_cost as u64
    );
    assert_eq!(
        env.get_token_amount(exchange_seller)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT * 2 - TOKEN_COST * 2
    );
    assert_eq!(
        env.get_token_amount(exchange_buyer)
            .await
            .unwrap_or_default(),
        STARTING_AMOUNT * 2 + (foreign_cost * 2.) as u64
    );
}
