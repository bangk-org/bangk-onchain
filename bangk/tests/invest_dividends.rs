// File: tests-onchain-main/tests/invest_dividends.rs
// Project: bangk-onchain
// Creation date: Friday 15 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

use bangk::{
    state::clients::Investment,
    state::{dividends_tracker::DividendsTracker, pda::BangkPda as _, projects::Periodicity},
};
use borsh::BorshDeserialize;
use chrono::{DateTime, Months, Utc};
use common::commands::{
    add_client, add_exchange, add_project, add_stable_mint, create_client_investment,
    create_client_investment_exchange, end_project, end_project_with_exchange, launch_project,
    mint_stable, reset_next_payment,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;

use crate::common::{
    commands::{pay_dividends, pay_dividends_with_exchange, transfer_invest},
    environment::Environment,
};

mod common;
const CLIENT1: &str = "Client 1";
const CLIENT2: &str = "Client 2";
const CLIENT3: &str = "Client 3";
const CLIENT4: &str = "Client 4";
const PROJECT: &str = "Test";
const CURRENCY_DEF: &str = "EuroBangk";
const CURRENCY_ALT: &str = "DollarBangk";
const SYMBOL: &str = "TST";
const URI: &str = "https://bangk.app/test";
const INTEREST_RATE: f64 = 7.5;
const TOKEN_VALUE: u32 = 1;
const PERIODICITY: Periodicity = Periodicity::BiAnnually;
const RISK: u8 = 4;
const AMOUNT: u64 = 10;
const EXCHANGE_RATE_DEF_ALT: f64 = 1.111;
const EXCHANGE_RATE_ALT_DEF: f64 = 0.9001;

/// Setup the testing environment for the integration tests that follow
/// This one adds clients investement to test cancelling or closing a project.
async fn setup() -> (
    Environment,
    Pubkey,
    (Pubkey, Pubkey),
    (Pubkey, Pubkey),
    (Pubkey, Pubkey),
    (Pubkey, Pubkey),
) {
    let mut env = Environment::get().await;
    add_stable_mint(&mut env, CURRENCY_DEF, 2).await;
    add_stable_mint(&mut env, CURRENCY_ALT, 2).await;
    let _ = add_client(&mut env, CLIENT1).await;
    let _ = add_client(&mut env, CLIENT2).await;
    let _ = add_client(&mut env, CLIENT3).await;
    let _ = add_client(&mut env, CLIENT4).await;
    let _ = mint_stable(&mut env, CLIENT1, CURRENCY_DEF, 10_000).await;
    let _ = mint_stable(&mut env, CLIENT2, CURRENCY_DEF, 10_000).await;
    let _ = mint_stable(&mut env, CLIENT3, CURRENCY_ALT, 10_000).await;
    let _ = mint_stable(&mut env, CLIENT4, CURRENCY_ALT, 10_000).await;
    add_exchange(&mut env, CURRENCY_DEF, 10_000).await;
    add_exchange(&mut env, CURRENCY_ALT, 10_000).await;

    let mint = add_project(
        &mut env,
        (PROJECT, SYMBOL, URI),
        CURRENCY_DEF,
        INTEREST_RATE,
        TOKEN_VALUE,
        PERIODICITY,
        RISK,
    )
    .await;

    let (client1_token, client1_record, _) =
        create_client_investment(&mut env, SYMBOL, CLIENT1, CURRENCY_DEF, AMOUNT).await;
    let (client2_token, client2_record, _) =
        create_client_investment(&mut env, SYMBOL, CLIENT2, CURRENCY_DEF, AMOUNT).await;

    let (client3_token, client3_record, _) = create_client_investment_exchange(
        &mut env,
        SYMBOL,
        CLIENT3,
        CURRENCY_DEF,
        CURRENCY_ALT,
        AMOUNT,
        EXCHANGE_RATE_ALT_DEF,
    )
    .await;
    let (client4_token, client4_record, _) = create_client_investment_exchange(
        &mut env,
        SYMBOL,
        CLIENT4,
        CURRENCY_DEF,
        CURRENCY_ALT,
        AMOUNT,
        EXCHANGE_RATE_ALT_DEF,
    )
    .await;

    launch_project(&mut env, SYMBOL).await;
    reset_next_payment(&mut env, &mint, Utc::now()).await;

    (
        env,
        mint,
        (client1_token, client1_record),
        (client2_token, client2_record),
        (client3_token, client3_record),
        (client4_token, client4_record),
    )
}

#[tokio::test]
async fn default() {
    let (mut env, mint_project, client1, client2, _, _) = setup().await;
    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;

    let project_stable = env.accounts[format!("{} ({})", SYMBOL, CURRENCY_DEF).as_str()].0;
    let client1_stable = env.accounts[format!("{} ({})", CLIENT1, CURRENCY_DEF).as_str()].0;
    let client2_stable = env.accounts[format!("{} ({})", CLIENT2, CURRENCY_DEF).as_str()].0;
    let project = env.get_project(mint_project).await;
    let record_client1 = env.get_account(client1.1).await.unwrap();
    let record_client2 = env.get_account(client2.1).await.unwrap();
    let record_client1 = Investment::try_from_slice(record_client1.data.as_slice()).unwrap();
    let record_client2 = Investment::try_from_slice(record_client2.data.as_slice()).unwrap();
    let tracker = DividendsTracker::get_address(&[&mint_project]).0;
    let tracker =
        DividendsTracker::try_from_slice(&env.get_account(tracker).await.unwrap().data).unwrap();

    assert_eq!(
        env.get_token_amount(client1_stable)
            .await
            .unwrap_or_default(),
        9_075
    );
    assert_eq!(
        env.get_token_amount(client2_stable)
            .await
            .unwrap_or_default(),
        9_075
    );
    assert_eq!(
        env.get_token_amount(project_stable)
            .await
            .unwrap_or_default(),
        3_850
    );

    assert_eq!(record_client1.last_payment, project.next_payment);
    assert_eq!(record_client2.last_payment, project.next_payment);
    assert_eq!(tracker.payment_date, project.next_payment);
    assert_eq!(tracker.total_clients, 4);
    assert_eq!(tracker.paid_clients, 2);
}

#[tokio::test]
#[should_panic]
async fn double_payment() {
    let (mut env, _, _, _, _, _) = setup().await;

    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;

    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn payment_too_soon() {
    let (mut env, _, _, _, _, _) = setup().await;

    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;

    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;
}

#[tokio::test]
async fn exchange() {
    let (mut env, mint_project, _, _, _, _) = setup().await;

    pay_dividends_with_exchange(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CURRENCY_ALT,
        &[CLIENT3, CLIENT4],
        INTEREST_RATE,
        EXCHANGE_RATE_DEF_ALT,
    )
    .await;

    let project = env.get_project(mint_project).await;
    let project_stable = env.accounts[format!("{} ({})", SYMBOL, CURRENCY_DEF).as_str()].0;
    let client3_stable = env.accounts[format!("{} ({})", CLIENT3, CURRENCY_ALT).as_str()].0;
    let client4_stable = env.accounts[format!("{} ({})", CLIENT4, CURRENCY_ALT).as_str()].0;
    let exchange_project = env.accounts[format!("Bangk ({})", CURRENCY_DEF).as_str()].0;
    let exchange_clients = env.accounts[format!("Bangk ({})", CURRENCY_ALT).as_str()].0;
    assert_eq!(
        env.get_token_amount(project_stable)
            .await
            .unwrap_or_default(),
        3_850
    );
    assert_eq!(
        env.get_token_amount(client3_stable)
            .await
            .unwrap_or_default(),
        8_972
    );
    assert_eq!(
        env.get_token_amount(client4_stable)
            .await
            .unwrap_or_default(),
        8_972
    );
    assert_eq!(
        env.get_token_amount(exchange_project)
            .await
            .unwrap_or_default(),
        8_150
    );
    assert_eq!(
        env.get_token_amount(exchange_clients)
            .await
            .unwrap_or_default(),
        12_056
    );

    let tracker = DividendsTracker::get_address(&[&mint_project]).0;
    let tracker =
        DividendsTracker::try_from_slice(&env.get_account(tracker).await.unwrap().data).unwrap();
    assert_eq!(tracker.payment_date, project.next_payment);
    assert_eq!(tracker.total_clients, 4);
    assert_eq!(tracker.paid_clients, 2);
}

#[tokio::test]
#[should_panic]
async fn exchange_rate_zero() {
    let (mut env, _, _, _, _, _) = setup().await;

    pay_dividends_with_exchange(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CURRENCY_ALT,
        &[CLIENT3, CLIENT4],
        INTEREST_RATE,
        0.,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn exchange_not_needed() {
    let (mut env, _, _, _, _, _) = setup().await;

    pay_dividends_with_exchange(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
        EXCHANGE_RATE_DEF_ALT,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn wrong_client_currency() {
    let (mut env, _, _, _, _, _) = setup().await;
    let _ = mint_stable(&mut env, CLIENT3, CURRENCY_DEF, 10_000).await;

    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT3],
        INTEREST_RATE,
    )
    .await;
}

#[tokio::test]
#[should_panic]
async fn interest_rate_zero() {
    let (mut env, _, _, _, _, _) = setup().await;
    pay_dividends(&mut env, SYMBOL, CURRENCY_DEF, &[CLIENT1, CLIENT2], 0.).await;
}

#[tokio::test]
#[should_panic]
async fn wrong_project_state() {
    let (mut env, mint_project, _, _, _, _) = setup().await;
    end_project(&mut env, "close", SYMBOL, CURRENCY_DEF, &[CLIENT1, CLIENT2]).await;
    end_project_with_exchange(
        &mut env,
        "close",
        SYMBOL,
        CURRENCY_DEF,
        CURRENCY_ALT,
        &[CLIENT3, CLIENT4],
        EXCHANGE_RATE_DEF_ALT,
    )
    .await;
    assert_eq!(env.get_mint_state(mint_project).await.supply, 0);

    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;
}

#[tokio::test]
async fn update_next_payment() {
    let (mut env, mint_project, _client1, _client2, _client3, _client4) = setup().await;
    let next_payment = Utc::now()
        .checked_add_months(Months::new(6))
        .unwrap()
        .timestamp();

    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;

    pay_dividends_with_exchange(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CURRENCY_ALT,
        &[CLIENT3, CLIENT4],
        INTEREST_RATE,
        EXCHANGE_RATE_DEF_ALT,
    )
    .await;

    let project = env.get_project(mint_project).await;
    assert!(
        i64::abs(project.last_payment - Utc::now().timestamp()) < 20,
        "last payment: {}",
        DateTime::<Utc>::from_timestamp(project.last_payment, 0).unwrap()
    );
    assert!(i64::abs(project.next_payment - next_payment) < 20);

    let tracker = DividendsTracker::get_address(&[&mint_project]).0;
    let tracker =
        DividendsTracker::try_from_slice(&env.get_account(tracker).await.unwrap().data).unwrap();
    assert_eq!(tracker.payment_date, project.last_payment);
    assert_eq!(tracker.total_clients, 4);
    assert_eq!(tracker.paid_clients, 0);
    assert!(project.next_payment > project.last_payment);
}

#[tokio::test]
#[should_panic]
async fn pending_payments_blocks() {
    let (mut env, _, _, _, _, _) = setup().await;
    pay_dividends(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        &[CLIENT1, CLIENT2],
        INTEREST_RATE,
    )
    .await;

    transfer_invest(
        &mut env,
        SYMBOL,
        CURRENCY_DEF,
        CLIENT1,
        CLIENT2,
        AMOUNT / 2,
        500,
    )
    .await;
}
