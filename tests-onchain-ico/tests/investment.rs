// File: tests-onchain-ico/tests/investment.rs
// Project: bangk-solana
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 11:24:26
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::too_many_lines)]
#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::indexing_slicing)]

use bangk_ico::{
    instruction::user_investment,
    investment::UserInvestmentPda,
    unvesting::{UnvestingScheme, UnvestingType},
};
use bangk_onchain_common::{errors::BangkError, pda::PdaType};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};

use crate::common::PROGRAM_ID;

pub mod common;

const INVESTED_AMOUNT: u64 = 1_000_000_000;

#[tokio::test]
async fn default() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let Ok(instruction) = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
    assert_eq!(pda.pda_type, PdaType::IcoInvestment);
    assert_eq!(pda.investment.user, user);
    assert_eq!(pda.investment.investments.len(), 1);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
}

#[tokio::test]
async fn add_investment() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let Ok(instruction) = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction.clone()], &["API"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let res2 = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
    assert_eq!(pda.investment.investments.len(), 2);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
    assert_eq!(
        pda.investment.investments[1].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[1].custom_rule, None);
    assert_eq!(pda.investment.investments[1].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[1].amount_released, 0);
}

#[tokio::test]
async fn two_investments1() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let Ok(instruction1) = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1.clone()], &["API"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let Ok(instruction2) = user_investment(
        &api,
        &user,
        UnvestingType::AdvisersPartners,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res2 = env.execute_transaction(&[instruction2], &["API"]).await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
    assert_eq!(pda.investment.investments.len(), 2);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
    assert_eq!(
        pda.investment.investments[1].kind,
        UnvestingType::AdvisersPartners
    );
    assert_eq!(pda.investment.investments[1].custom_rule, None);
    assert_eq!(pda.investment.investments[1].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[1].amount_released, 0);
}

#[tokio::test]
async fn two_investments2() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let Ok(instruction1) = user_investment(
        &api,
        &user,
        UnvestingType::PublicSells2,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1.clone()], &["API"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let Ok(instruction2) = user_investment(
        &api,
        &user,
        UnvestingType::PublicSells3,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res2 = env.execute_transaction(&[instruction2], &["API"]).await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
    assert_eq!(pda.investment.investments.len(), 2);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::PublicSells2
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
    assert_eq!(
        pda.investment.investments[1].kind,
        UnvestingType::PublicSells3
    );
    assert_eq!(pda.investment.investments[1].custom_rule, None);
    assert_eq!(pda.investment.investments[1].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[1].amount_released, 0);
}

#[tokio::test]
async fn custom_scheme() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let custom_scheme = UnvestingScheme {
        kind: UnvestingType::AdvisersPartners,
        start: 10,
        duration: 12,
        initial_unvesting: 40_000,
        weekly_unvesting: 40_000,
        final_unvesting: 20_000,
    };

    let Ok(instruction) = user_investment(
        &api,
        &user,
        UnvestingType::AdvisersPartners,
        Some(custom_scheme),
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
    assert_eq!(pda.pda_type, PdaType::IcoInvestment);
    assert_eq!(pda.investment.user, user);
    assert_eq!(pda.investment.investments.len(), 1);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::AdvisersPartners
    );
    assert_eq!(
        pda.investment.investments[0].custom_rule,
        Some(custom_scheme)
    );
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
}

#[tokio::test]
async fn invalid_custom_scheme() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let custom_scheme1 = UnvestingScheme {
        kind: UnvestingType::AdvisersPartners,
        start: 10,
        duration: 12,
        initial_unvesting: 40_000,
        weekly_unvesting: 40_000,
        final_unvesting: 10_000,
    };

    let Ok(instruction1) = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        Some(custom_scheme1),
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res1 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res1.is_err_and(|err| err == BangkError::InvalidUnvestingDefinition),
        "there was an unexpected error in the instruction"
    );

    let custom_scheme2 = UnvestingScheme {
        kind: UnvestingType::AdvisersPartners,
        start: 10,
        duration: 12,
        initial_unvesting: 30_000,
        weekly_unvesting: 40_000,
        final_unvesting: 10_000,
    };

    let Ok(instruction2) = user_investment(
        &api,
        &user,
        UnvestingType::AdvisersPartners,
        Some(custom_scheme2),
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res2 = env.execute_transaction(&[instruction2], &["API"]).await;
    assert!(
        res2.is_err_and(|err| err == BangkError::InvalidUnvestingDefinition),
        "there was an unexpected error in the instruction"
    );
}

// TODO: add invest after launch test
