// File: tests-onchain-ico/tests/cancel_investment.rs
// Project: bangk-solana
// Creation date: Thursday 13 June 2024
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
#![allow(clippy::integer_division)]

use bangk_ico::{
    instruction::{cancel_investment, user_investment},
    investment::UserInvestmentPda,
    unvesting::UnvestingType,
};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};

use crate::common::PROGRAM_ID;

pub mod common;

const INVESTED_AMOUNT: u64 = 1_000_000_000;

#[tokio::test]
async fn one_investment() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    // Create the investment
    let Ok(instruction1) = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res1 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    assert!(env.get_account(&investment_pda).await.is_some());

    // Delete the investment
    let admin2 = env.wallets["Admin 2"].pubkey();
    let Ok(instruction2) = cancel_investment(
        &api,
        &admin2,
        &user,
        UnvestingType::TeamFounders,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res2 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2"])
        .await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );
    assert!(env.get_account(&investment_pda).await.is_none());
}

#[tokio::test]
async fn partial() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    // Create the investment
    let Ok(instruction1) = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    ) else {
        panic!("could not create instruction");
    };
    let res1 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    assert!(env.get_account(&investment_pda).await.is_some());

    // Delete the investment
    let admin2 = env.wallets["Admin 2"].pubkey();
    let Ok(instruction2) = cancel_investment(
        &api,
        &admin2,
        &user,
        UnvestingType::TeamFounders,
        INVESTED_AMOUNT / 2,
    ) else {
        panic!("could not create instruction");
    };
    let res2 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2"])
        .await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let Some(pda): Option<UserInvestmentPda> = env.from_account(&investment_pda).await else {
        panic!("could not load the investment PDA");
    };
    assert_eq!(
        pda.investment.investments[0].amount_bought,
        INVESTED_AMOUNT / 2
    );
}

#[tokio::test]
async fn two_same_kind() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    // Create the investment
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
    let res2 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(&user, &PROGRAM_ID);
    assert!(env.get_account(&investment_pda).await.is_some());

    // Delete the investment
    let admin2 = env.wallets["Admin 2"].pubkey();
    let Ok(instruction2) = cancel_investment(
        &api,
        &admin2,
        &user,
        UnvestingType::TeamFounders,
        INVESTED_AMOUNT * 2,
    ) else {
        panic!("could not create instruction");
    };
    let res3 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2"])
        .await;
    assert!(
        res3.is_ok(),
        "there was an unexpected error in the instruction"
    );
    assert!(env.get_account(&investment_pda).await.is_none());
}
