// File: tests-onchain-ico/tests/unvesting.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 25 July 2024 @ 20:46:42
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]
#![allow(clippy::integer_division)]
#![allow(clippy::unwrap_used)]

pub mod common;

use bangk_ico::{vesting_release, ConfigurationPda, UnvestingType, UserInvestmentPda};
use bangk_onchain_common::Error;
use common::{add_investment, get_unvesting_def, launch_tokens, PROGRAM_ID};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

const INVEST_TYPE1: UnvestingType = UnvestingType::TeamFounders;
const INVEST_TYPE2: UnvestingType = UnvestingType::PublicSells2;
const INVEST_TYPE3: UnvestingType = UnvestingType::PublicSells3;
const INVESTED_AMOUNT: u64 = 1_000_000_000_000;
const WEEK: i64 = 7 * 86_400;

#[tokio::test]
async fn before_launch() {
    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;

    let api = env.wallets["API"].pubkey();

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_err_and(|err| err == Error::IcoUnvestBeforeLaunch),
        "there was an unexpected error in the instruction"
    );
}

#[tokio::test]
async fn before_initial() {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme.start) * WEEK + 3600,
        INVESTED_AMOUNT,
    )
    .await;

    let api = env.wallets["API"].pubkey();

    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Check results
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
    assert_eq!(
        pda.investment
            .investments
            .iter()
            .find(|invest| invest.kind == INVEST_TYPE1)
            .map(|invest| invest.amount_released),
        Some(0)
    );
    assert_eq!(env.get_token_amount(&user_ata).await, None);
    assert_eq!(
        env.get_token_amount(&invested_ata).await,
        Some(INVESTED_AMOUNT)
    );
}

#[tokio::test]
async fn initial() {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme.start) * WEEK - 3600,
        INVESTED_AMOUNT,
    )
    .await;

    let api = env.wallets["API"].pubkey();

    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Check results
    let target = INVESTED_AMOUNT * u64::from(scheme.initial_unvesting) / 100_000;
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
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
        env.get_token_amount(&invested_ata).await,
        Some(INVESTED_AMOUNT - target)
    );
}

#[tokio::test]
async fn one_week_in() {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.start) + 1) * WEEK - 3600,
        INVESTED_AMOUNT,
    )
    .await;

    let api = env.wallets["API"].pubkey();

    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Check results
    let target =
        INVESTED_AMOUNT * u64::from(scheme.initial_unvesting + scheme.weekly_unvesting) / 100_000;
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
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
        env.get_token_amount(&invested_ata).await,
        Some(INVESTED_AMOUNT - target)
    );
}

#[tokio::test]
async fn last_week() {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.duration) - 1) * WEEK - 3600,
        INVESTED_AMOUNT,
    )
    .await;

    let api = env.wallets["API"].pubkey();

    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Check results
    let target = INVESTED_AMOUNT
        * (u64::from(scheme.initial_unvesting)
            + u64::from(scheme.duration - scheme.start - 1) * u64::from(scheme.weekly_unvesting))
        / 100_000;
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
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
        env.get_token_amount(&invested_ata).await,
        Some(INVESTED_AMOUNT - target)
    );
}

#[tokio::test]
async fn after_end() {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme.duration) * WEEK - 3600,
        INVESTED_AMOUNT,
    )
    .await;

    let api = env.wallets["API"].pubkey();

    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Check results
    let target = INVESTED_AMOUNT;
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
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
        env.get_token_amount(&invested_ata).await,
        Some(INVESTED_AMOUNT - target)
    );
}

#[tokio::test]
async fn two_types() {
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
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);

    let mut env = common::init_with_mint().await;
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE2, None).await;
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE3, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - i64::from(scheme1.start + 2) * WEEK - 3600,
        INVESTED_AMOUNT * 2,
    )
    .await;

    let api = env.wallets["API"].pubkey();

    let user_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Check results
    let target1 = INVESTED_AMOUNT
        * u64::from(scheme1.initial_unvesting + scheme1.weekly_unvesting * 2)
        / 100_000;
    let target2 = INVESTED_AMOUNT
        * u64::from(scheme2.initial_unvesting + scheme2.weekly_unvesting * 2)
        / 100_000;
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
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
        env.get_token_amount(&invested_ata).await,
        Some(INVESTED_AMOUNT * 2 - target1 - target2)
    );
}

#[tokio::test]
async fn non_admin_payer() {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.start) + 1) * WEEK - 3600,
        INVESTED_AMOUNT,
    )
    .await;

    let payer = env.add_wallet("User").await;

    // Release the tokens
    let Ok(instruction1) = vesting_release(&payer, &user) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction1], &["User"]).await;
    assert!(
        res.is_err_and(|err| err == Error::InvalidSigner),
        "there was an unexpected error in the instruction"
    );
}

#[tokio::test]
async fn twice() {
    let scheme = get_unvesting_def()
        .iter()
        .find(|rule| rule.kind == INVEST_TYPE1)
        .copied()
        .unwrap();

    let mut env = common::init_with_mint().await;
    let user = Pubkey::new_unique();
    add_investment(&mut env, &user, INVESTED_AMOUNT, INVEST_TYPE1, None).await;
    launch_tokens(
        &mut env,
        chrono::Utc::now().timestamp() - (i64::from(scheme.start) + 1) * WEEK - 3600,
        INVESTED_AMOUNT,
    )
    .await;

    let api = env.wallets["API"].pubkey();

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);

    // Release the tokens
    let Ok(instruction1) = vesting_release(&api, &user) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1.clone()], &["API"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Check results
    let Some(first): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };

    // Try to release a second time
    // Release the tokens
    let res2 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let Some(second): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };

    first
        .investment
        .investments
        .iter()
        .zip(second.investment.investments.iter())
        .for_each(|(before, after)| assert_eq!(before.amount_released, after.amount_released));
}
