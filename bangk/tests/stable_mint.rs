// File: tests-onchain-main/tests/stable_mint.rs
// Project: bangk-onchain
// Creation date: Friday 08 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use bangk::instruction::{BangkInstruction, TokenAmountArgs};
use common::commands::{add_client, add_stable_mint};
use solana_program::{instruction::AccountMeta, system_program};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

use crate::common::{commands::mint_stable, environment::Environment};
mod common;

const CLIENT: &str = "Client 1";
const CURRENCY: &str = "EuroBangk";

/// Setup the testing environment for the integration tests that follow
async fn setup() -> Environment {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, CURRENCY, 2).await;
    let _ = add_client(&mut env, CLIENT).await;
    env
}

#[tokio::test]
async fn stable_mint() {
    let mut env = setup().await;
    let ata = mint_stable(&mut env, CLIENT, CURRENCY, 100).await;
    assert_eq!(env.get_token_amount(ata).await.unwrap_or_default(), 100);
}

#[tokio::test]
#[should_panic]
async fn zero() {
    let mut env = setup().await;
    let _ = mint_stable(&mut env, CLIENT, CURRENCY, 0).await;
}

#[tokio::test]
#[should_panic]
async fn mismatch_ata_currency() {
    let currency = "SomethingElse";
    let mut env = setup().await;
    add_stable_mint(&mut env, currency, 2).await;
    let ata = mint_stable(&mut env, CLIENT, currency, 10_000).await;

    // Get accounts
    let mint_foreign = env.accounts[CURRENCY].0;
    let client = env.wallets[CLIENT].pubkey();
    let payload = BangkInstruction::MintStableCoin(TokenAmountArgs { amount: 10 });

    env.execute_failing_transaction(
        &payload,
        &[
            AccountMeta::new(env.payer.pubkey(), true),
            AccountMeta::new(mint_foreign, false),
            AccountMeta::new(client, true),
            AccountMeta::new(ata, false),
            AccountMeta::new(system_program::id(), false),
            AccountMeta::new(spl_token_2022::id(), false),
            AccountMeta::new(spl_associated_token_account::id(), false),
        ],
        &[CLIENT],
    )
    .await;
}
