// File: tests-onchain-main/tests/stable_burn.rs
// Project: bangk-onchain
// Creation date: Wednesday 06 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use bangk::instruction::{BangkInstruction, BurnStableCoinsArgs};
use common::commands::{add_client, add_stable_mint, mint_stable};
use solana_program::{instruction::AccountMeta, pubkey::Pubkey};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

use crate::common::{commands::burn_stable, environment::Environment};

mod common;

const CLIENT: &str = "Client 1";
const CURRENCY: &str = "EuroBangk";

/// Setup the testing environment for the integration tests that follow
async fn setup() -> (Environment, Pubkey) {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, CURRENCY, 2).await;
    let _ = add_client(&mut env, CLIENT).await;
    let ata = mint_stable(&mut env, CLIENT, CURRENCY, 10_000).await;
    (env, ata)
}

#[tokio::test]
async fn default() {
    let (mut env, ata) = setup().await;
    burn_stable(&mut env, CLIENT, CURRENCY, 10, true).await;
    assert_eq!(env.get_token_amount(ata).await.unwrap_or_default(), 9_990);
}

#[tokio::test]
#[should_panic]
async fn zero() {
    let (mut env, _) = setup().await;
    burn_stable(&mut env, CLIENT, CURRENCY, 0, true).await;
}

// Account is empty but still exists
#[tokio::test]
async fn max() {
    let (mut env, ata) = setup().await;
    burn_stable(&mut env, CLIENT, CURRENCY, 10_000, false).await;
    assert_eq!(env.get_token_amount(ata).await.unwrap_or_default(), 0);
}

// Account is empty and has been removed
#[tokio::test]
async fn max_and_close() {
    let (mut env, ata) = setup().await;
    burn_stable(&mut env, CLIENT, CURRENCY, 10_000, true).await;
    assert!(env.get_account(ata).await.is_none());
}

// Close empty account
#[tokio::test]
async fn close_empty() {
    let (mut env, ata) = setup().await;
    burn_stable(&mut env, CLIENT, CURRENCY, 10_000, false).await;
    assert!(env.get_account(ata).await.is_some());
    burn_stable(&mut env, CLIENT, CURRENCY, 0, true).await;
    assert!(env.get_account(ata).await.is_none());
}

#[tokio::test]
#[should_panic]
async fn too_much() {
    let (mut env, _) = setup().await;
    burn_stable(&mut env, CLIENT, CURRENCY, 100_000, true).await;
}

#[tokio::test]
#[should_panic]
async fn account_does_not_exist() {
    let (mut env, _) = setup().await;
    let payer = env.payer.pubkey();
    let mint_key = env.accounts[CURRENCY].0;
    let ata = Pubkey::new_unique();

    let payload = BangkInstruction::BurnStableCoin(BurnStableCoinsArgs {
        amount: 10,
        close_empty: true,
    });

    env.execute_failing_transaction(
        &payload,
        &[
            AccountMeta::new(payer, true),
            AccountMeta::new(mint_key, false),
            AccountMeta::new(ata, false),
            AccountMeta::new(spl_token_2022::id(), false),
        ],
        &[],
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn mismatch_ata_currency() {
    let (mut env, _) = setup().await;
    let payer = env.payer.pubkey();
    let mint_key = env.accounts[CURRENCY].0;
    add_stable_mint(&mut env, "SomethingElse", 2).await;
    let ata = mint_stable(&mut env, CLIENT, "SomethingElse", 10_000).await;

    let payload = BangkInstruction::BurnStableCoin(BurnStableCoinsArgs {
        amount: 10,
        close_empty: true,
    });

    env.execute_failing_transaction(
        &payload,
        &[
            AccountMeta::new(payer, true),
            AccountMeta::new(mint_key, false),
            AccountMeta::new(ata, false),
            AccountMeta::new(spl_token_2022::id(), false),
        ],
        &[],
    )
    .await;
}
