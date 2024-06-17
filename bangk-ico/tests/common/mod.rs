// File: tests-onchain-ico/tests/common/mod.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 24 July 2024 @ 18:59:03
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]

use bangk_ico::{
    create_mint, initialize, launch_bgk, process_instruction, transfer_from_reserve,
    user_investment, UnvestingScheme, UnvestingType,
};
use solana_program_test::processor;
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};
use tests_utilities::onchain::Environment;

pub const PROGRAM_ID: Pubkey =
    solana_program::pubkey!("BKPrg3v1Y3SJmK1uSvEpgccAx3LNwr6yWkGzSDttioFv");
pub const TOTAL_BGK_TOKENS: u64 = 177_000_000_000_000;

/// Get the default unvesting schemes definitions
#[must_use]
pub fn get_unvesting_def() -> Vec<UnvestingScheme> {
    vec![
        UnvestingScheme {
            kind: UnvestingType::TeamFounders,
            start: 52,
            duration: 157,
            initial_unvesting: 10000,
            weekly_unvesting: 800,
            final_unvesting: 6800,
        },
        UnvestingScheme {
            kind: UnvestingType::AdvisersPartners,
            start: 26,
            duration: 52,
            initial_unvesting: 10000,
            weekly_unvesting: 3500,
            final_unvesting: 2500,
        },
        UnvestingScheme {
            kind: UnvestingType::PrivateSells,
            start: 2,
            duration: 41,
            initial_unvesting: 10000,
            weekly_unvesting: 2300,
            final_unvesting: 2600,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells1,
            start: 2,
            duration: 41,
            initial_unvesting: 10000,
            weekly_unvesting: 2300,
            final_unvesting: 2600,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells2,
            start: 2,
            duration: 28,
            initial_unvesting: 10000,
            weekly_unvesting: 3500,
            final_unvesting: 2500,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells3,
            start: 2,
            duration: 15,
            initial_unvesting: 10000,
            weekly_unvesting: 7000,
            final_unvesting: 6000,
        },
    ]
}

/// Default initialization of the ICO program
///
/// # Panics
/// If the environment couldn't be set correctly.
pub async fn init_default() -> Environment {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
    let Some(api_key) = env.wallets.get("API") else {
        panic!("no API key in the environment");
    };
    let api_pub = api_key.pubkey();

    let admin1 = env.add_wallet("Admin 1").await;
    let admin2 = env.add_wallet("Admin 2").await;
    let admin3 = env.add_wallet("Admin 3").await;
    let admin4 = env.add_wallet("Admin 4").await;

    let Ok(instruction) = initialize(
        &api_pub,
        get_unvesting_def(),
        &api_pub,
        &admin1,
        &admin2,
        &admin3,
        &admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    env
}

/// Initializes the testing environment with the mint created and the tokens minted
///
/// # Panics
/// If the environment couldn't be set correctly.
pub async fn init_with_mint() -> Environment {
    let mut env = init_default().await;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let Ok(instruction) = create_mint(&admin1, &admin2, &admin3) else {
        panic!("could not create instruction");
    };
    // println!("Instruction: {instruction:#?}");
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    env
}

/// Add an investment for a user.
///
/// # Panics
/// If the investment couldn't be done.
pub async fn add_investment(
    env: &mut Environment,
    user: &Pubkey,
    amount: u64,
    kind: UnvestingType,
    custom_rule: Option<UnvestingScheme>,
) {
    println!("adding investment for wallet {user}");
    let api = env.wallets["API"].pubkey();

    let Ok(instruction) = user_investment(&api, user, kind, custom_rule, amount) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );
}

/// Launch the BGK tokens
///
/// # Panics
/// If the launch couldn't be set.
pub async fn launch_tokens(env: &mut Environment, timestamp: i64, amount: u64) {
    println!("launching BGK tokens at date {timestamp}");
    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();

    // Create the investment
    let Ok(instruction) = launch_bgk(&api, &admin2, &admin4, timestamp, amount) else {
        panic!("could not create instruction");
    };
    let res = env
        .execute_transaction(&[instruction], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );
}

/// Transfer tokens from the reserve to another ATA
///
/// # Panics
/// If the transfer couldn't be done.
pub async fn transfer_bgk_from_reserve(env: &mut Environment, user: &Pubkey, amount: u64) {
    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    // Transfer the tokens
    let Ok(instruction1) = transfer_from_reserve(&api, &admin2, &admin4, user, amount) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
}
