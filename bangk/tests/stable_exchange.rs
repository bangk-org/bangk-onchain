// File: tests-onchain-main/tests/stable_exchange.rs
// Project: bangk-onchain
// Creation date: Monday 18 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use common::commands::{add_client, add_exchange, add_stable_mint, mint_stable};
use solana_program_test::*;

use crate::common::{
    commands::{burn_stable, create_ata, exchange_stable},
    environment::Environment,
};

mod common;

const CLIENT: &str = "Client 1";
const SOURCE: &str = "EuroBangk";
const TARGET1: &str = "DollarBangk";
const TARGET2: &str = "YenBangk";

/// Setup the testing environment for the integration tests that follow
async fn setup() -> Environment {
    let mut env = Environment::get().await;
    println!("adding mint {SOURCE}");
    add_stable_mint(&mut env, SOURCE, 2).await;
    println!("adding mint {TARGET1}");
    add_stable_mint(&mut env, TARGET1, 2).await;
    println!("adding mint {TARGET2}");
    add_stable_mint(&mut env, TARGET2, 0).await;
    println!("adding client {CLIENT}");
    let _ = add_client(&mut env, CLIENT).await;
    println!("adding Bangk");
    let _ = add_client(&mut env, "Bangk").await;
    let _ = mint_stable(&mut env, CLIENT, SOURCE, 10_000).await;
    println!("adding exchange {SOURCE}");
    add_exchange(&mut env, SOURCE, 20_000).await;
    println!("adding exchange {TARGET1}");
    add_exchange(&mut env, TARGET1, 20_000).await;
    println!("adding exchange {TARGET2}");
    add_exchange(&mut env, TARGET2, 20_000).await;
    env
}

// 100 dollar tokens received, so 90.91 euro tokens received
#[tokio::test]
async fn default() {
    let mut env = setup().await;
    let ata_target = create_ata(&mut env, CLIENT, TARGET1, false).await;
    exchange_stable(&mut env, CLIENT, SOURCE, TARGET1, 100, 1.1).await;

    let exchange_source = env.accounts[format!("Bangk ({})", SOURCE).as_str()].0;
    let exchange_target = env.accounts[format!("Bangk ({})", TARGET1).as_str()].0;
    let ata_source = env.accounts[format!("{} ({})", CLIENT, SOURCE).as_str()].0;
    assert_eq!(
        env.get_token_amount(ata_source).await.unwrap_or_default(),
        9_909
    );
    assert_eq!(
        env.get_token_amount(exchange_source)
            .await
            .unwrap_or_default(),
        20_091
    );
    assert_eq!(
        env.get_token_amount(ata_target).await.unwrap_or_default(),
        100
    );
    assert_eq!(
        env.get_token_amount(exchange_target)
            .await
            .unwrap_or_default(),
        19_900
    );
}

// 150 yens received, so 1 euro spent (= 100 tokens)
#[tokio::test]
async fn target_has_less_decimals() {
    let mut env = setup().await;
    let ata_target = create_ata(&mut env, CLIENT, TARGET2, false).await;
    exchange_stable(&mut env, CLIENT, SOURCE, TARGET2, 150, 150.).await;

    let exchange_source = env.accounts[format!("Bangk ({})", SOURCE).as_str()].0;
    let exchange_target = env.accounts[format!("Bangk ({})", TARGET2).as_str()].0;
    let ata_source = env.accounts[format!("{} ({})", CLIENT, SOURCE).as_str()].0;
    assert_eq!(
        env.get_token_amount(ata_target).await.unwrap_or_default(),
        150
    );
    assert_eq!(
        env.get_token_amount(exchange_target)
            .await
            .unwrap_or_default(),
        19_850
    );
    assert_eq!(
        env.get_token_amount(ata_source).await.unwrap_or_default(),
        9_900
    );
    assert_eq!(
        env.get_token_amount(exchange_source)
            .await
            .unwrap_or_default(),
        20_100
    );
}

// 1 euro received (= 100 tokens), means 151 yens paid (= 151 tokens)
#[tokio::test]
async fn target_has_more_decimals() {
    let mut env = setup().await;
    let ata_source = create_ata(&mut env, CLIENT, TARGET2, false).await;
    let _ = mint_stable(&mut env, CLIENT, TARGET2, 151).await;
    exchange_stable(&mut env, CLIENT, TARGET2, SOURCE, 100, 1. / 150.).await;

    let exchange_source = env.accounts[format!("Bangk ({})", TARGET2).as_str()].0;
    let exchange_target = env.accounts[format!("Bangk ({})", SOURCE).as_str()].0;
    let ata_target = env.accounts[format!("{} ({})", CLIENT, SOURCE).as_str()].0;
    assert_eq!(
        env.get_token_amount(ata_source).await.unwrap_or_default(),
        0
    );
    assert_eq!(
        env.get_token_amount(exchange_source)
            .await
            .unwrap_or_default(),
        20_151
    );
    assert_eq!(
        env.get_token_amount(ata_target).await.unwrap_or_default(),
        10_100
    );
    assert_eq!(
        env.get_token_amount(exchange_target)
            .await
            .unwrap_or_default(),
        19_900
    );
}

#[tokio::test]
#[should_panic]
async fn nonexistent_dest() {
    let mut env = setup().await;
    exchange_stable(&mut env, CLIENT, SOURCE, TARGET1, 100, 1.1).await;
}

#[tokio::test]
#[should_panic]
async fn amount_zero() {
    let mut env = setup().await;
    let _ = create_ata(&mut env, CLIENT, TARGET1, false).await;
    exchange_stable(&mut env, CLIENT, SOURCE, TARGET1, 0, 1.1).await;
}

#[tokio::test]
#[should_panic]
async fn exchange_zero() {
    let mut env = setup().await;
    let _ = create_ata(&mut env, CLIENT, TARGET1, false).await;
    exchange_stable(&mut env, CLIENT, SOURCE, TARGET1, 100, 0.).await;
}

#[tokio::test]
#[should_panic]
async fn exchange_inufficient_funds() {
    let mut env = setup().await;
    let _ = create_ata(&mut env, CLIENT, TARGET1, false).await;
    exchange_stable(&mut env, CLIENT, SOURCE, TARGET1, 100_000, 1.1).await;
}

#[tokio::test]
#[should_panic]
async fn not_enough_on_relay() {
    let mut env = setup().await;
    burn_stable(&mut env, "Bangk", TARGET1, 19_999, true).await;
    let _ = create_ata(&mut env, CLIENT, TARGET1, false).await;
    exchange_stable(&mut env, CLIENT, SOURCE, TARGET1, 100, 1.1).await;
}
