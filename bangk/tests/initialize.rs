// File: tests-onchain-main/tests/initialize.rs
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

use crate::common::{commands::add_stable_mint, environment::Environment};
use bangk::{
    instruction::{create_stable_coin, BangkInstruction, CreateStableCoinArgs},
    utils::get_stable_mint_address,
};
use solana_program::{instruction::AccountMeta, system_program};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::{
    default_account_state::DefaultAccountState, metadata_pointer::MetadataPointer,
    permanent_delegate::PermanentDelegate,
};

mod common;

#[tokio::test]
async fn default() {
    let mut env = Environment::get().await;
    let name = "EuroBangk";
    let symbol = "EUB";
    let uri = "https://i.imgur.com/aRQPusR.png";
    env.execute_transaction(
        &[create_stable_coin(name, symbol, uri, 2_u8).unwrap()],
        &["bangk", "delegate"],
    )
    .await;
    let (mint_key, _) = get_stable_mint_address(symbol);

    assert_eq!(env.get_mint_state(mint_key).await.supply, 0);
    assert_eq!(
        env.get_mint_state_with_extensions::<PermanentDelegate>(mint_key)
            .await
            .delegate,
        Some(env.delegate.pubkey()).try_into().unwrap()
    );
    assert_eq!(
        env.get_mint_state_with_extensions::<DefaultAccountState>(mint_key)
            .await
            .state,
        1,
    );
    let metadata_pointer = env
        .get_mint_state_with_extensions::<MetadataPointer>(mint_key)
        .await;
    assert_eq!(
        metadata_pointer.authority,
        Some(env.payer.pubkey()).try_into().unwrap()
    );
    assert_eq!(
        metadata_pointer.metadata_address,
        Some(mint_key).try_into().unwrap()
    );
    let metadata = env.get_mint_metadata(mint_key).await;
    assert_eq!(metadata.mint, mint_key);
    assert_eq!(metadata.name, name);
    assert_eq!(metadata.symbol, symbol);
    assert_eq!(metadata.uri, uri);
}

#[tokio::test]
#[should_panic]
async fn empty_payload() {
    let mut environment = Environment::get().await;
    let payload: &[u8] = &[];
    environment
        .execute_failing_transaction(&payload, &[], &[])
        .await;
}

#[tokio::test]
#[should_panic]
async fn wrong_payload() {
    let mut environment = Environment::get().await;
    let payload: &[u8] = &[255, 12, 8, 18, 1, 9];
    environment
        .execute_failing_transaction(&payload, &[], &[])
        .await;
}

#[tokio::test]
#[should_panic]
async fn double_init() {
    let mut env = Environment::get().await;
    let name = "EuroBangk";
    let _ = add_stable_mint(&mut env, name, 2).await;
    let _ = add_stable_mint(&mut env, name, 2).await;
}

#[tokio::test]
#[should_panic]
async fn stable_wrong_seed() {
    let mut environment = Environment::get().await;
    let (mint_key, _) = get_stable_mint_address("BangkEuro");
    environment
        .execute_failing_transaction(
            &BangkInstruction::CreateStableCoin(CreateStableCoinArgs {
                currency: String::from("EuroBangk"),
                symbol: String::from("EUB"),
                uri: String::from("https://bangk.app/euro_bangk"),
                decimals: 2_u8,
            }),
            &[
                AccountMeta::new(environment.payer.pubkey(), true),
                AccountMeta::new(mint_key, false),
                AccountMeta::new(system_program::ID, false),
                AccountMeta::new(spl_token_2022::id(), false),
            ],
            &[],
        )
        .await;
}
