// File: bangk-ico/tests/cancel_investment.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 21 August 2024 @ 19:33:07
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic_in_result_fn)]
#![allow(clippy::integer_division)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

use bangk_ico::{cancel_investment, user_investment, UnvestingType, UserInvestmentPda};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};

use crate::common::PROGRAM_ID;

pub mod common;

const INVESTED_AMOUNT: u64 = 1_000_000_000;

#[tokio::test]
async fn one_investment() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    // Create the investment
    let instruction1 = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    )?;
    let res1 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    assert!(env.get_account(&investment_pda).await.is_some());

    // Delete the investment
    let admin2 = env.wallets["Admin 2"].pubkey();
    let instruction2 = cancel_investment(
        &api,
        &admin2,
        &user,
        UnvestingType::TeamFounders,
        INVESTED_AMOUNT,
    )?;
    let res2 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2"])
        .await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );
    assert!(env.get_account(&investment_pda).await.is_none());

    Ok(())
}

#[tokio::test]
async fn partial() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    // Create the investment
    let instruction1 = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction1], &["API"]).await?;
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    assert!(env.get_account(&investment_pda).await.is_some());

    // Delete the investment
    let admin2 = env.wallets["Admin 2"].pubkey();
    let instruction2 = cancel_investment(
        &api,
        &admin2,
        &user,
        UnvestingType::TeamFounders,
        INVESTED_AMOUNT / 2,
    )?;
    env.execute_transaction(&[instruction2], &["API", "Admin 2"])
        .await?;
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("failed to get investment PDA")?;
    assert_eq!(
        pda.investment.investments[0].amount_bought,
        INVESTED_AMOUNT / 2
    );

    Ok(())
}

#[tokio::test]
async fn two_same_kind() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    // Create the investment
    let instruction1 = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction1.clone()], &["API"])
        .await?;
    env.execute_transaction(&[instruction1], &["API"]).await?;
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    assert!(env.get_account(&investment_pda).await.is_some());

    // Delete the investment
    let admin2 = env.wallets["Admin 2"].pubkey();
    let instruction2 = cancel_investment(
        &api,
        &admin2,
        &user,
        UnvestingType::TeamFounders,
        INVESTED_AMOUNT * 2,
    )?;
    env.execute_transaction(&[instruction2], &["API", "Admin 2"])
        .await?;
    assert!(env.get_account(&investment_pda).await.is_none());

    Ok(())
}
