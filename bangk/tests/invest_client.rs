// File: tests-onchain-main/tests/invest_client.rs
// Project: bangk-onchain
// Creation date: Monday 04 December 2023
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
    clients::Investment, dividends_tracker::DividendsTracker, pda::BangkPda as _,
    projects::Periodicity,
};
use bangk_onchain_common::pda::PdaType;
use borsh::BorshDeserialize;
use common::{
    commands::{
        add_client, add_exchange, add_project, add_stable_mint, create_client_investment,
        create_client_investment_exchange, mint_stable,
    },
    environment::Environment,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::default_account_state::DefaultAccountState;

use crate::common::commands::create_ata;

mod common;

const CLIENT: &str = "Client 1";
const PROJECT: &str = "Test";
const CURRENCY_PROJECT: &str = "EuroBangk";
const CURRENCY_CLIENT: &str = "DollarBangk";
const SYMBOL: &str = "TST";
const URI: &str = "https://bangk.app/test";
const INTEREST_RATE: f64 = 7.5;
const TOKEN_VALUE: u32 = 1;
const PERIODICITY: Periodicity = Periodicity::Daily;
const RISK: u8 = 2;
const AMOUNT: u64 = 10;
const COST: u64 = 1000;
const EXCHANGE_RATE_PROJECT_CLIENTS: f64 = 1.111;
const EXCHANGE_RATE_CLIENTS_PROJECT: f64 = 0.9001;
const STARTING_STABLE: u64 = 10_000;

/// Setup the testing environment for the integration tests that follow
/// This one adds clients investement to test cancelling or closing a project.
///
/// # Parameters
/// * `project_currency` - Currency used by the project,
/// * `client_currency` - Currency used by the clients.
async fn setup(
    project_currency: &str,
    client_currency: &str,
) -> (Environment, Pubkey, (Pubkey, Pubkey)) {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, project_currency, 2).await;
    if project_currency != client_currency {
        add_stable_mint(&mut env, client_currency, 2).await;
    }
    let _ = add_client(&mut env, PROJECT).await;
    let _ = add_client(&mut env, CLIENT).await;
    let _ = mint_stable(&mut env, CLIENT, client_currency, STARTING_STABLE).await;
    if project_currency != client_currency {
        add_exchange(&mut env, CURRENCY_PROJECT, 10_000).await;
        add_exchange(&mut env, CURRENCY_CLIENT, 10_000).await;
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
        println!("Creating default investment");
        let (client1_token, client1_record, _) =
            create_client_investment(&mut env, SYMBOL, CLIENT, client_currency, AMOUNT).await;

        (env, mint, (client1_token, client1_record))
    } else {
        println!("Creating exchange investment");
        let (client1_token, client1_record, _) = create_client_investment_exchange(
            &mut env,
            SYMBOL,
            CLIENT,
            project_currency,
            client_currency,
            AMOUNT,
            EXCHANGE_RATE_CLIENTS_PROJECT,
        )
        .await;

        (env, mint, (client1_token, client1_record))
    }
}

#[tokio::test]
async fn default() {
    let (mut env, mint_project, client) = setup(CURRENCY_PROJECT, CURRENCY_PROJECT).await;

    let client_key = env.wallets[CLIENT].pubkey();
    let client_stable = env.accounts[format!("{} ({})", CLIENT, CURRENCY_PROJECT).as_str()].0;
    let project_stable = env.accounts[&format!("{} ({})", SYMBOL, CURRENCY_PROJECT)].0;
    let mint_state = env.get_mint_state(mint_project).await;
    assert_eq!(mint_state.supply, AMOUNT);
    assert_eq!(
        env.get_mint_state_with_extensions::<DefaultAccountState>(mint_project)
            .await
            .state,
        2,
    );
    assert_eq!(
        env.get_token_amount(client_stable)
            .await
            .unwrap_or_default(),
        STARTING_STABLE - COST
    );
    assert_eq!(
        env.get_token_amount(project_stable)
            .await
            .unwrap_or_default(),
        COST
    );
    assert_eq!(
        env.get_token_amount(client.0).await.unwrap_or_default(),
        AMOUNT
    );
    let record =
        Investment::try_from_slice(&env.get_account(client.1).await.unwrap().data).unwrap();
    assert_eq!(record.pda_type, PdaType::UserProjectInvestment);
    assert_eq!(record.client, client_key.to_string());
    assert_eq!(record.project, mint_project.to_string());
    assert_eq!(record.ata, client_stable.to_string());
    assert!(i64::abs(chrono::Utc::now().timestamp() - record.creation) < 60);
    assert_eq!(record.last_payment, 0);

    let tracker = DividendsTracker::get_address(&[&mint_project]).0;
    let tracker =
        DividendsTracker::try_from_slice(&env.get_account(tracker).await.unwrap().data).unwrap();
    assert_eq!(tracker.total_clients, 1);
}

#[tokio::test]
async fn exchange() {
    let (mut env, mint_project, client) = setup(CURRENCY_PROJECT, CURRENCY_CLIENT).await;

    let client_stable = env.accounts[format!("{} ({})", CLIENT, CURRENCY_CLIENT).as_str()].0;
    let project_stable = env.accounts[format!("{} ({})", SYMBOL, CURRENCY_PROJECT).as_str()].0;
    let exchange_client = env.accounts[format!("Bangk ({})", CURRENCY_CLIENT).as_str()].0;
    let exchange_project = env.accounts[format!("Bangk ({})", CURRENCY_PROJECT).as_str()].0;

    let mint_state = env.get_mint_state(mint_project).await;
    assert_eq!(mint_state.supply, AMOUNT);
    let cost = (COST as f64 * EXCHANGE_RATE_PROJECT_CLIENTS) as u64;
    assert_eq!(
        env.get_token_amount(client.0).await.unwrap_or_default(),
        AMOUNT
    );
    assert_eq!(
        env.get_token_amount(project_stable)
            .await
            .unwrap_or_default(),
        COST
    );
    assert_eq!(
        env.get_token_amount(client_stable)
            .await
            .unwrap_or_default(),
        STARTING_STABLE - cost
    );
    assert_eq!(
        env.get_token_amount(exchange_client)
            .await
            .unwrap_or_default(),
        STARTING_STABLE + cost
    );
    assert_eq!(
        env.get_token_amount(exchange_project)
            .await
            .unwrap_or_default(),
        STARTING_STABLE - COST
    );
    let record =
        Investment::try_from_slice(&env.get_account(client.1).await.unwrap().data).unwrap();
    assert_eq!(record.ata, client_stable.to_string());

    let tracker = DividendsTracker::get_address(&[&mint_project]).0;
    let tracker =
        DividendsTracker::try_from_slice(&env.get_account(tracker).await.unwrap().data).unwrap();
    assert_eq!(tracker.total_clients, 1);
}

#[tokio::test]
#[should_panic]
async fn unecessary_exchange() {
    let (mut env, _, _) = setup(CURRENCY_PROJECT, CURRENCY_PROJECT).await;
    add_stable_mint(&mut env, CURRENCY_CLIENT, 2).await;
    let _ = mint_stable(&mut env, "Bangk", CURRENCY_PROJECT, STARTING_STABLE).await;
    let _ = mint_stable(&mut env, "Bangk", CURRENCY_CLIENT, STARTING_STABLE).await;
    let _ = create_client_investment_exchange(
        &mut env,
        SYMBOL,
        CLIENT,
        CURRENCY_PROJECT,
        CURRENCY_PROJECT,
        AMOUNT,
        EXCHANGE_RATE_CLIENTS_PROJECT,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn exchange_needed() {
    let (mut env, _, _) = setup(CURRENCY_PROJECT, CURRENCY_CLIENT).await;
    let _ = create_ata(&mut env, PROJECT, CURRENCY_CLIENT, false).await;
    let _ = create_client_investment(&mut env, SYMBOL, CLIENT, CURRENCY_CLIENT, AMOUNT).await;
}

#[tokio::test]
async fn double_invest() {
    let (mut env, mint_project, client) = setup(CURRENCY_PROJECT, CURRENCY_PROJECT).await;

    let _ = create_client_investment(&mut env, SYMBOL, CLIENT, CURRENCY_PROJECT, AMOUNT).await;

    let client_stable = env.accounts[format!("{} ({})", CLIENT, CURRENCY_PROJECT).as_str()].0;
    let project_stable = env.accounts[&format!("{} ({})", SYMBOL, CURRENCY_PROJECT)].0;
    let mint_state = env.get_mint_state(mint_project).await;
    let tracker = DividendsTracker::get_address(&[&mint_project]).0;
    let tracker =
        DividendsTracker::try_from_slice(&env.get_account(tracker).await.unwrap().data).unwrap();

    assert_eq!(mint_state.supply, AMOUNT * 2);
    assert_eq!(
        env.get_mint_state_with_extensions::<DefaultAccountState>(mint_project)
            .await
            .state,
        2,
    );
    assert_eq!(
        env.get_token_amount(client_stable)
            .await
            .unwrap_or_default(),
        STARTING_STABLE - COST * 2
    );
    assert_eq!(
        env.get_token_amount(project_stable)
            .await
            .unwrap_or_default(),
        COST * 2
    );
    assert_eq!(
        env.get_token_amount(client.0).await.unwrap_or_default(),
        AMOUNT * 2
    );
    assert_eq!(tracker.total_clients, 1);
}
