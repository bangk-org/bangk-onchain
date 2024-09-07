// File: tests-onchain-main/tests/stable_transfer.rs
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

use common::commands::{add_client, add_stable_mint, mint_stable};
use solana_program_test::*;

use crate::common::{
    commands::{add_exchange, create_ata, transfer_stable},
    environment::Environment,
};

mod common;

const CLIENT1: &str = "Client 1";
const CLIENT2: &str = "Client 2";
const SOURCE: &str = "EuroBangk";
const TARGET: &str = "DollarBangk";

/// Setup the testing environment for the integration tests that follow
async fn setup() -> Environment {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, SOURCE, 2).await;
    add_stable_mint(&mut env, TARGET, 2).await;
    let _ = add_client(&mut env, CLIENT1).await;
    let _ = add_client(&mut env, CLIENT2).await;
    println!("adding {CLIENT1}");
    let _ = mint_stable(&mut env, CLIENT1, SOURCE, 10_000).await;
    println!("adding Bangk {SOURCE}");
    let _ = add_exchange(&mut env, SOURCE, 10_000).await;
    println!("adding Bangk {TARGET}");
    let _ = add_exchange(&mut env, TARGET, 10_000).await;
    env
}

#[tokio::test]
async fn default() {
    let mut env = setup().await;
    let _ = mint_stable(&mut env, CLIENT2, SOURCE, 10_000).await;
    let ata_to = create_ata(&mut env, CLIENT2, SOURCE, false).await;
    transfer_stable(&mut env, CLIENT1, CLIENT2, SOURCE, 100).await;

    let ata_from = env.accounts[format!("{} ({})", CLIENT1, SOURCE).as_str()].0;
    assert_eq!(
        env.get_token_amount(ata_from).await.unwrap_or_default(),
        9_900
    );
    assert_eq!(
        env.get_token_amount(ata_to).await.unwrap_or_default(),
        10_100
    );
}

#[tokio::test]
#[should_panic]
async fn too_many() {
    let mut env = setup().await;
    let _ = create_ata(&mut env, CLIENT2, SOURCE, false).await;
    transfer_stable(&mut env, CLIENT1, CLIENT2, SOURCE, 11_000).await;
}

#[tokio::test]
#[should_panic]
async fn zero() {
    let mut env = setup().await;
    let _ = create_ata(&mut env, CLIENT2, SOURCE, false).await;
    transfer_stable(&mut env, CLIENT1, CLIENT2, SOURCE, 0).await;
}

#[tokio::test]
#[should_panic]
async fn nonexistent_dest() {
    let mut env = setup().await;
    transfer_stable(&mut env, CLIENT1, CLIENT2, SOURCE, 100).await;
}
