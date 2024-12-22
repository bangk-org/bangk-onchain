// File: bangk-stable/tests/common/mod.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:55:18
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

use bangk_stable::{create_stable_coin, initialize, process_instruction};
use solana_program_test::processor;
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};
use tests_utilities::onchain::Environment;

pub const PROGRAM_ID: Pubkey =
    solana_program::pubkey!("BKPrg5rXBCXMEJPnL2K8DaFEcua1e1SkeLUGqSQhkj6U");

/// Default initialization of the ICO program
///
/// # Errors
/// If the initialization failed
pub async fn init_default() -> Result<Environment> {
    let mut env =
        Environment::new(PROGRAM_ID, "bangk_stable", processor!(process_instruction)).await;
    let api_key = env
        .wallets
        .get("API")
        .ok_or("no API key in the environment")?;
    let api_pub = api_key.pubkey();

    let admin1 = env.add_wallet("Admin 1").await;
    let admin2 = env.add_wallet("Admin 2").await;
    let admin3 = env.add_wallet("Admin 3").await;
    let admin4 = env.add_wallet("Admin 4").await;

    let instruction = initialize(&api_pub, &api_pub, &admin1, &admin2, &admin3, &admin4)?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    Ok(env)
}

/// Initializes the testing environment with the mint created and the tokens minted
///
/// # Errors
/// If the initialization failed
pub async fn init_with_mint(
    currency: &str,
    symbol: &str,
    uri: &str,
    decimals: u8,
) -> Result<Environment> {
    let mut env = init_default().await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let instruction1 = create_stable_coin(
        &admin1,
        &admin2,
        &admin3,
        currency.to_owned(),
        symbol.to_owned(),
        uri.to_owned(),
        decimals,
    )?;
    env.execute_transaction(&[instruction1], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    Ok(env)
}
