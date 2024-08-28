// File: bangk-ico/tests/launch.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 21 August 2024 @ 19:33:07
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

pub mod common;
use bangk_ico::{launch_bgk, ConfigurationPda, UnvestingType};
use bangk_onchain_common::Error as BangkError;
use common::add_investment;
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

const TIMESTAMP: i64 = 19829;
const AMOUNT: u64 = 50_000_000_000_000;

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();

    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);

    // Add a dummy investment
    add_investment(
        &mut env,
        &Pubkey::new_unique(),
        AMOUNT,
        UnvestingType::TeamFounders,
        None,
    )
    .await?;

    let instruction1 = launch_bgk(&api, &admin2, &admin4, TIMESTAMP)?;
    env.execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await?;
    assert!(env
        .from_account::<ConfigurationPda>(&config_pda)
        .await
        .is_some_and(|config| config.launch_date == TIMESTAMP));

    Ok(())
}

#[tokio::test]
async fn double_launch() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();

    // Add a dummy investment
    add_investment(
        &mut env,
        &Pubkey::new_unique(),
        AMOUNT,
        UnvestingType::TeamFounders,
        None,
    )
    .await?;

    // Create the investment
    let instruction1 = launch_bgk(&api, &admin2, &admin4, TIMESTAMP)?;
    env.execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await?;
    let instruction2 = launch_bgk(&api, &admin2, &admin4, TIMESTAMP + 2)?;
    let res = env
        .execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::BGKTokenAlreadyLaunched),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}
