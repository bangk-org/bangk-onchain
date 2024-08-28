// File: bangk-ico/tests/unvesting.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 22 August 2024 @ 12:24:51
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]
#![allow(clippy::integer_division)]
#![allow(clippy::unwrap_used)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

pub mod common;

use std::{error, result};

use bangk_ico::{vesting_release, UnvestingType, UserInvestmentPda, WalletType};
use bangk_onchain_common::Error as BangkError;
use common::{add_investment, get_unvesting_def, launch_tokens, PROGRAM_ID, TOTAL_ICO_TOKENS};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

const INVEST_TYPE1: UnvestingType = UnvestingType::TeamFounders;
const INVEST_TYPE2: UnvestingType = UnvestingType::PublicSells2;
const INVEST_TYPE3: UnvestingType = UnvestingType::PublicSells3;
const INVESTED_AMOUNT: u64 = 1_000_000_000_000;
const WEEK: i64 = 7 * 86_400;

#[tokio::test]
async fn before_launch() -> Result<()> {
    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;

    let api = env.wallets["API"].pubkey();

    // Release the tokens
    let instruction1 = vesting_release(&api, &user)?;
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_err_and(|err| err == BangkError::IcoUnvestBeforeLaunch),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn before_initial() -> Result<()> {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme.start) * WEEK + 3600,
    )
    .await?;

    let api = env.wallets["API"].pubkey();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let ico_pda = WalletType::Ico.get_pda().0;

    // Release the tokens
    let instruction1 = vesting_release(&api, &user)?;
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Check results
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE1)
            .map(|invest| invest.amount_released),
        Some(0)
    );
    assert_eq!(env.get_token_amount(&user_ata).await, None);
    assert_eq!(env.get_token_amount(&ico_pda).await, Some(TOTAL_ICO_TOKENS));

    Ok(())
}

#[tokio::test]
async fn initial() -> Result<()> {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme.start) * WEEK - 3600,
    )
    .await?;

    let api = env.wallets["API"].pubkey();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let ico_pda = WalletType::Ico.get_pda().0;

    // Release the tokens
    let instruction1 = vesting_release(&api, &user)?;
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Check results
    let target = INVESTED_AMOUNT * u64::from(scheme.initial_unvesting) / 100_000;
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE1)
            .map(|invest| invest.amount_released),
        Some(target)
    );
    assert_eq!(env.get_token_amount(&user_ata).await, Some(target));
    assert_eq!(
        env.get_token_amount(&ico_pda).await,
        Some(TOTAL_ICO_TOKENS - target)
    );

    Ok(())
}

#[tokio::test]
async fn one_week_in() -> Result<()> {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.start) + 1) * WEEK - 3600,
    )
    .await?;

    let api = env.wallets["API"].pubkey();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let ico_pda = WalletType::Ico.get_pda().0;

    // Release the tokens
    let instruction1 = vesting_release(&api, &user)?;
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Check results
    let target =
        INVESTED_AMOUNT * u64::from(scheme.initial_unvesting + scheme.weekly_unvesting) / 100_000;
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE1)
            .map(|invest| invest.amount_released),
        Some(target)
    );
    assert_eq!(env.get_token_amount(&user_ata).await, Some(target));
    assert_eq!(
        env.get_token_amount(&ico_pda).await,
        Some(TOTAL_ICO_TOKENS - target)
    );

    Ok(())
}

#[tokio::test]
async fn last_week() -> Result<()> {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.duration) - 1) * WEEK - 3600,
    )
    .await?;

    let api = env.wallets["API"].pubkey();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let ico_pda = WalletType::Ico.get_pda().0;

    // Release the tokens
    let instruction = vesting_release(&api, &user)?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    // Check results
    let target = INVESTED_AMOUNT
        * (u64::from(scheme.initial_unvesting)
            + u64::from(scheme.duration - scheme.start - 1) * u64::from(scheme.weekly_unvesting))
        / 100_000;
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE1)
            .map(|invest| invest.amount_released),
        Some(target)
    );
    assert_eq!(env.get_token_amount(&user_ata).await, Some(target));
    assert_eq!(
        env.get_token_amount(&ico_pda).await,
        Some(TOTAL_ICO_TOKENS - target)
    );

    Ok(())
}

#[tokio::test]
async fn after_end() -> Result<()> {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme.duration) * WEEK - 3600,
    )
    .await?;

    let api = env.wallets["API"].pubkey();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let ico_pda = WalletType::Ico.get_pda().0;

    // Release the tokens
    let instruction1 = vesting_release(&api, &user)?;
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Check results
    let target = INVESTED_AMOUNT;
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE1)
            .map(|invest| invest.amount_released),
        Some(target)
    );
    assert_eq!(env.get_token_amount(&user_ata).await, Some(target));
    assert_eq!(
        env.get_token_amount(&ico_pda).await,
        Some(TOTAL_ICO_TOKENS - target)
    );

    Ok(())
}

#[tokio::test]
async fn two_types() -> Result<()> {
    let scheme1 = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE2)
        .copied()
        .unwrap();
    let scheme2 = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE3)
        .copied()
        .unwrap();
    let user = Pubkey::new_unique();
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);

    let mut env = common::init_with_mint().await?;
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE2, None).await?;
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE3, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme1.start + 2) * WEEK - 3600,
    )
    .await?;

    let api = env.wallets["API"].pubkey();

    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let ico_pda = WalletType::Ico.get_pda().0;

    // Release the tokens
    let instruction1 = vesting_release(&api, &user)?;
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Check results
    let target1 = INVESTED_AMOUNT
        * u64::from(scheme1.initial_unvesting + scheme1.weekly_unvesting * 2)
        / 100_000;
    let target2 = INVESTED_AMOUNT
        * u64::from(scheme2.initial_unvesting + scheme2.weekly_unvesting * 2)
        / 100_000;
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(pda.investment.investments.len(), 2);
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE2)
            .map(|invest| invest.amount_released),
        Some(target1)
    );
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE3)
            .map(|invest| invest.amount_released),
        Some(target2)
    );
    assert_eq!(
        env.get_token_amount(&user_ata).await,
        Some(target1 + target2)
    );
    assert_eq!(
        env.get_token_amount(&ico_pda).await,
        Some(TOTAL_ICO_TOKENS - target1 - target2)
    );

    Ok(())
}

#[tokio::test]
async fn non_admin_payer() -> Result<()> {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.start) + 1) * WEEK - 3600,
    )
    .await?;

    let payer = env.add_wallet("User").await;

    // Release the tokens
    let instruction1 = vesting_release(&payer, &user)?;
    let res = env.execute_transaction(&[instruction1], &["User"]).await;
    assert!(
        res.is_err_and(|err| err == BangkError::InvalidSigner),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn twice() -> Result<()> {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await?;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await?;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.start) + 1) * WEEK - 3600,
    )
    .await?;

    let api = env.wallets["API"].pubkey();

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);

    // Release the tokens
    let instruction1 = vesting_release(&api, &user)?;
    env.execute_transaction(&[instruction1.clone()], &["API"])
        .await?;

    // Check results
    let first: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;

    // Try to release a second time
    // Release the tokens
    env.execute_transaction(&[instruction1], &["API"]).await?;
    let second: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;

    first
        .investment
        .investments
        .iter()
        .zip(second.investment.investments.iter())
        .for_each(|(before, after)| assert_eq!(before.amount_released, after.amount_released));

    Ok(())
}
