// File: tests-onchain-main/tests/invest_init.rs
// Project: bangk-onchain
// Creation date: Friday 24 November 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use bangk::state::{
    dividends_tracker::DividendsTracker,
    pda::BangkPda as _,
    projects::{Periodicity, ProjectStatus},
};
use bangk_onchain_common::pda::PdaType;
use borsh::BorshDeserialize;
use chrono::{Months, Utc};
use common::commands::{add_project, add_stable_mint};
use solana_program::{program_option::COption, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::{
    default_account_state::DefaultAccountState, permanent_delegate::PermanentDelegate,
};

use crate::common::{commands::launch_project, environment::Environment};

mod common;

const PROJECT: &str = "Test";
const CURRENCY: &str = "EuroBangk";
const SYMBOL: &str = "TST";
const URI: &str = "https://bangk.app/test";
const INTEREST_RATE: f64 = 7.5;
const TOKEN_VALUE: u32 = 100;
const PERIODICITY: Periodicity = Periodicity::Monthly;
const RISK: u8 = 2;

/// Setup the testing environment for the integration tests that follow
async fn setup() -> (Environment, Pubkey) {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, CURRENCY, 2).await;

    let mint = add_project(
        &mut env,
        (PROJECT, SYMBOL, URI),
        CURRENCY,
        INTEREST_RATE,
        TOKEN_VALUE,
        PERIODICITY,
        RISK,
    )
    .await;
    (env, mint)
}

#[tokio::test]
async fn register_project() {
    let (mut env, mint_project) = setup().await;

    let tracker = DividendsTracker::get_address(&[&mint_project]).0;
    let mint_state = env.get_mint_state(mint_project).await;
    assert_eq!(
        mint_state.mint_authority,
        COption::Some(env.delegate.pubkey())
    );
    assert_eq!(mint_state.supply, 0);
    assert_eq!(mint_state.decimals, 0);
    assert!(mint_state.is_initialized);
    assert_eq!(
        mint_state.freeze_authority,
        COption::Some(env.delegate.pubkey())
    );
    // Extensions check
    let delegate = env
        .get_mint_state_with_extensions::<PermanentDelegate>(mint_project)
        .await;
    let key: Option<Pubkey> = delegate.delegate.into();
    assert_eq!(key.unwrap(), env.delegate.pubkey());
    let default_status = env
        .get_mint_state_with_extensions::<DefaultAccountState>(mint_project)
        .await;
    assert_eq!(default_status.state, 2); // 2 is frozen
    let project = env.get_project(mint_project).await;
    assert_eq!(project.name, PROJECT);
    assert_eq!(project.symbol, SYMBOL);
    assert_eq!(project.uri, URI);
    assert_eq!(project.interest_rate, (INTEREST_RATE * 1e6) as u32);
    assert_eq!(project.last_payment, 0);
    assert_eq!(project.next_payment, 0);
    assert_eq!(project.token_value, TOKEN_VALUE);
    assert_eq!(project.payment_periodicity, PERIODICITY);
    assert_eq!(project.risk_assessment, RISK);
    assert_eq!(project.status, ProjectStatus::Open);

    let tracker =
        DividendsTracker::try_from_slice(&env.get_account(tracker).await.unwrap().data).unwrap();
    assert_eq!(tracker.project, mint_project.to_string());
    assert_eq!(tracker.pda_type, PdaType::ProjectDividendsTracker);
    assert_eq!(tracker.payment_date, 0);
    assert_eq!(tracker.total_clients, 0);
    assert_eq!(tracker.paid_clients, 0);
}

#[tokio::test]
#[should_panic]
async fn duplicate() {
    let (mut env, _) = setup().await;
    let _ = add_project(
        &mut env,
        (PROJECT, SYMBOL, URI),
        CURRENCY,
        INTEREST_RATE,
        TOKEN_VALUE,
        PERIODICITY,
        RISK,
    )
    .await;
}

#[tokio::test]
async fn launch() {
    let (mut env, mint_project) = setup().await;
    let next_payment = Utc::now()
        .checked_add_months(Months::new(1))
        .unwrap()
        .timestamp();
    launch_project(&mut env, SYMBOL).await;

    let project_next_payment = env.get_project_next_payment(mint_project).await;
    assert!(i64::abs(project_next_payment - next_payment) < 2);
    assert_eq!(
        env.get_project_status(mint_project).await,
        ProjectStatus::Live
    );
}

#[tokio::test]
#[should_panic]
async fn double_launch() {
    let (mut env, _) = setup().await;
    launch_project(&mut env, SYMBOL).await;
    launch_project(&mut env, SYMBOL).await;
}
