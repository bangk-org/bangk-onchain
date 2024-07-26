// File: tests-onchain-main/tests/invest_projects_update.rs
// Project: bangk-onchain
// Creation date: Thursday 14 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

mod common;
use bangk::state::projects::{Periodicity, ProjectStatus};
use common::{
    commands::{
        add_client, add_exchange, add_project, add_stable_mint, create_client_investment,
        create_client_investment_exchange, launch_project, mint_stable,
    },
    environment::Environment,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;

use crate::common::commands::{end_project, end_project_with_exchange};

const CLIENT1: &str = "Client 1";
const CLIENT2: &str = "Client 2";
const PROJECT: &str = "Test";
const CURRENCY_PROJECT: &str = "EuroBangk";
const CURRENCY_CLIENTS: &str = "DollarBangk";
const SYMBOL: &str = "TST";
const URI: &str = "https://bangk.app/test";
const INTEREST_RATE: f64 = 7.5;
const TOKEN_VALUE: u32 = 1;
const PERIODICITY: Periodicity = Periodicity::Quarterly;
const RISK: u8 = 2;
const AMOUNT: u64 = 10;
const EXCHANGE_RATE_PROJECT_CLIENTS: f64 = 1.111;
const EXCHANGE_RATE_CLIENTS_PROJECT: f64 = 0.9001;

/// Setup the testing environment for the integration tests that follow
/// This one adds clients investement to test cancelling or closing a project.
///
/// # Parameters
/// * `project_currency` - Currency used by the project,
/// * `client_currency` - Currency used by the clients.
async fn setup(
    project_currency: &str,
    client_currency: &str,
) -> (Environment, Pubkey, (Pubkey, Pubkey), (Pubkey, Pubkey)) {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, project_currency, 2).await;
    if project_currency != client_currency {
        add_stable_mint(&mut env, client_currency, 2).await;
    }
    let _ = add_client(&mut env, PROJECT).await;
    let _ = add_client(&mut env, CLIENT1).await;
    let _ = add_client(&mut env, CLIENT2).await;
    let _ = add_client(&mut env, "Bangk").await;
    let _ = mint_stable(&mut env, CLIENT1, client_currency, 10_000).await;
    let _ = mint_stable(&mut env, CLIENT2, client_currency, 10_000).await;
    if project_currency != client_currency {
        add_exchange(&mut env, CURRENCY_PROJECT, 10_000).await;
        add_exchange(&mut env, CURRENCY_CLIENTS, 10_000).await;
    }

    let mint = add_project(
        &mut env,
        (PROJECT, SYMBOL, URI),
        CURRENCY_PROJECT,
        INTEREST_RATE,
        TOKEN_VALUE,
        PERIODICITY,
        RISK,
    )
    .await;

    if client_currency == project_currency {
        let (client1_token, client1_record, _) =
            create_client_investment(&mut env, SYMBOL, CLIENT1, client_currency, AMOUNT).await;
        let (client2_token, client2_record, _) =
            create_client_investment(&mut env, SYMBOL, CLIENT2, client_currency, AMOUNT).await;

        (
            env,
            mint,
            (client1_token, client1_record),
            (client2_token, client2_record),
        )
    } else {
        let (client1_token, client1_record, _) = create_client_investment_exchange(
            &mut env,
            SYMBOL,
            CLIENT1,
            project_currency,
            client_currency,
            AMOUNT,
            EXCHANGE_RATE_CLIENTS_PROJECT,
        )
        .await;
        let (client2_token, client2_record, _) = create_client_investment_exchange(
            &mut env,
            SYMBOL,
            CLIENT2,
            project_currency,
            client_currency,
            AMOUNT,
            EXCHANGE_RATE_CLIENTS_PROJECT,
        )
        .await;

        (
            env,
            mint,
            (client1_token, client1_record),
            (client2_token, client2_record),
        )
    }
}

#[tokio::test]
async fn cancel() {
    let (mut env, mint_project, client1, client2) = setup(CURRENCY_PROJECT, CURRENCY_PROJECT).await;

    end_project(
        &mut env,
        "cancel",
        SYMBOL,
        CURRENCY_PROJECT,
        &[CLIENT1, CLIENT2],
    )
    .await;

    let mint_state = env.get_mint_state(mint_project).await;
    assert_eq!(mint_state.supply, 0);
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT1, CURRENCY_PROJECT).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT2, CURRENCY_PROJECT).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(env.accounts[format!("{} ({})", SYMBOL, CURRENCY_PROJECT).as_str()].0)
            .await
            .unwrap_or_default(),
        0
    );
    assert!(env.get_account(client1.0).await.is_none());
    assert!(env.get_account(client1.1).await.is_none());
    assert!(env.get_account(client2.0).await.is_none());
    assert!(env.get_account(client2.1).await.is_none());

    assert_eq!(
        env.get_project_status(mint_project).await,
        ProjectStatus::Cancelled
    );
}

#[tokio::test]
async fn close() {
    let (mut env, mint_project, client1, client2) = setup(CURRENCY_PROJECT, CURRENCY_PROJECT).await;
    launch_project(&mut env, SYMBOL).await;

    end_project(
        &mut env,
        "close",
        SYMBOL,
        CURRENCY_PROJECT,
        &[CLIENT1, CLIENT2],
    )
    .await;

    let mint_state = env.get_mint_state(mint_project).await;
    assert_eq!(mint_state.supply, 0);
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT1, CURRENCY_PROJECT).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT2, CURRENCY_PROJECT).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(env.accounts[format!("{} ({})", SYMBOL, CURRENCY_PROJECT).as_str()].0)
            .await
            .unwrap_or_default(),
        0
    );
    assert!(env.get_account(client1.0).await.is_none());
    assert!(env.get_account(client1.1).await.is_none());
    assert!(env.get_account(client2.0).await.is_none());
    assert!(env.get_account(client2.1).await.is_none());

    assert_eq!(
        env.get_project_status(mint_project).await,
        ProjectStatus::Closed
    );
}

#[tokio::test]
async fn cancel_with_exchange() {
    let (mut env, mint_project, _, _) = setup(CURRENCY_PROJECT, CURRENCY_CLIENTS).await;

    end_project_with_exchange(
        &mut env,
        "cancel",
        SYMBOL,
        CURRENCY_PROJECT,
        CURRENCY_CLIENTS,
        &[CLIENT1, CLIENT2],
        EXCHANGE_RATE_PROJECT_CLIENTS,
    )
    .await;

    let mint_state = env.get_mint_state(mint_project).await;
    assert_eq!(mint_state.supply, 0);
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT1, CURRENCY_CLIENTS).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT2, CURRENCY_CLIENTS).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(env.accounts[format!("{} ({})", SYMBOL, CURRENCY_PROJECT).as_str()].0)
            .await
            .unwrap_or_default(),
        0
    );
}

#[tokio::test]
async fn close_with_exchange() {
    let (mut env, mint_project, _, _) = setup(CURRENCY_PROJECT, CURRENCY_CLIENTS).await;
    launch_project(&mut env, SYMBOL).await;

    end_project_with_exchange(
        &mut env,
        "close",
        SYMBOL,
        CURRENCY_PROJECT,
        CURRENCY_CLIENTS,
        &[CLIENT1, CLIENT2],
        EXCHANGE_RATE_PROJECT_CLIENTS,
    )
    .await;

    let mint_state = env.get_mint_state(mint_project).await;
    assert_eq!(mint_state.supply, 0);
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT1, CURRENCY_CLIENTS).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(
            env.accounts[format!("{} ({})", CLIENT2, CURRENCY_CLIENTS).as_str()].0
        )
        .await
        .unwrap_or_default(),
        10_000
    );
    assert_eq!(
        env.get_token_amount(env.accounts[format!("{} ({})", SYMBOL, CURRENCY_PROJECT).as_str()].0)
            .await
            .unwrap_or_default(),
        0
    );
}

#[tokio::test]
#[should_panic]
async fn cancel_no_exchange_needed() {
    let (mut env, _, _, _) = setup(CURRENCY_PROJECT, CURRENCY_PROJECT).await;
    let _ = mint_stable(&mut env, "Bangk", CURRENCY_PROJECT, 10_000).await;

    end_project_with_exchange(
        &mut env,
        "cancel",
        SYMBOL,
        CURRENCY_PROJECT,
        CURRENCY_PROJECT,
        &[CLIENT1, CLIENT2],
        EXCHANGE_RATE_PROJECT_CLIENTS,
    )
    .await;
}
