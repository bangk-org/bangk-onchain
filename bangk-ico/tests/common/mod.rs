// File: bangk-ico/tests/common/mod.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 22 August 2024 @ 12:45:28
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

use bangk_ico::{
    create_mint, initialize, launch_bgk, process_instruction, queue_transfer_from_internal_wallet,
    user_investment, UnvestingScheme, UnvestingType, WalletType,
};
use solana_program_test::processor;
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};
use tests_utilities::onchain::Environment;

pub const PROGRAM_ID: Pubkey =
    solana_program::pubkey!("BKPrg3v1Y3SJmK1uSvEpgccAx3LNwr6yWkGzSDttioFv");
pub const TOTAL_BGK_TOKENS: u64 = 177_000_000_000_000;
pub const TOTAL_RESERVE_TOKENS: u64 = 30_000_000_000_000;
pub const TOTAL_ICO_TOKENS: u64 = 50_000_000_000_000;

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
/// # Errors
/// If the initialization failed
pub async fn init_default() -> Result<Environment> {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
    let api_key = env
        .wallets
        .get("API")
        .ok_or("no API key in the environment")?;
    let api_pub = api_key.pubkey();

    let admin1 = env.add_wallet("Admin 1").await;
    let admin2 = env.add_wallet("Admin 2").await;
    let admin3 = env.add_wallet("Admin 3").await;
    let admin4 = env.add_wallet("Admin 4").await;

    let instruction = initialize(
        &api_pub,
        get_unvesting_def(),
        &api_pub,
        &admin1,
        &admin2,
        &admin3,
        &admin4,
    )?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    Ok(env)
}

/// Initializes the testing environment with the mint created and the tokens minted
///
/// # Errors
/// If the initialization failed
pub async fn init_with_mint() -> Result<Environment> {
    let mut env = init_default().await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let instruction1 = create_mint(&admin1, &admin2, &admin3)?;
    env.execute_transaction(&[instruction1], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    Ok(env)
}

/// Add an investment for a user.
///
/// # Errors
/// If the initialization failed
pub async fn add_investment(
    env: &mut Environment,
    user: &Pubkey,
    amount: u64,
    kind: UnvestingType,
    custom_rule: Option<UnvestingScheme>,
) -> Result<()> {
    println!("adding investment for wallet {user}");
    let api = env.wallets["API"].pubkey();

    let instruction = user_investment(&api, user, kind, custom_rule, amount)?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    Ok(())
}

/// Launch the BGK tokens
///
/// # Errors
/// If the instruction failed
pub async fn launch_tokens(env: &mut Environment, timestamp: i64) -> Result<()> {
    println!("launching BGK tokens at date {timestamp}");
    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();

    // Create the investment
    let instruction = launch_bgk(&api, &admin2, &admin4, timestamp)?;
    env.execute_transaction(&[instruction], &["API", "Admin 2", "Admin 4"])
        .await?;

    Ok(())
}

/// Transfer tokens from the reserve to another ATA
///
/// # Errors
/// If the instruction failed
pub async fn transfer_bgk_from_reserve(
    env: &mut Environment,
    user: &Pubkey,
    amount: u64,
) -> Result<()> {
    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    // Transfer the tokens
    let instruction1 = queue_transfer_from_internal_wallet(
        &api,
        &admin2,
        &admin4,
        user,
        WalletType::Reserve,
        amount,
    )?;
    env.execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await?;

    Ok(())
}
