// File: tests-onchain-main/tests/common/commands.rs
// Project: bangk-onchain
// Creation date: Wednesday 27 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(dead_code)]

use bangk::{
    instruction::*,
    state::{
        clients::Investment,
        mint_data::MintData as _,
        pda::BangkPda as _,
        projects::{Periodicity, Project, ProjectStatus},
        stable::StableMint,
    },
    utils::{get_ata_address, get_stable_mint_address},
};
use chrono::{DateTime, Utc};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::keypair_from_seed_phrase_and_passphrase, signer::Signer, transaction::Transaction,
};
use spl_token_metadata_interface::{instruction::update_field, state::Field::Key};

use super::environment::Environment;

/// Adds a new stable coin mint
///
/// # Parameters
/// * `env` - Testing environment,
/// * `currency` - Name of the coin/currency to add,
/// * `decimals` - Number of decimals for the currency,
///
/// # Return
/// * Pubkey of the new mint
pub async fn add_stable_mint(env: &mut Environment, currency: &str, decimals: u8) -> Pubkey {
    let (mint_key, _) = get_stable_mint_address(currency);

    env.accounts.insert(String::from(currency), (mint_key, 0));
    env.execute_transaction(
        &[create_stable_coin(currency, currency, "uri", decimals).unwrap()],
        &["delegate"],
    )
    .await;
    mint_key
}

/// Adds a new client in the testing environment
///
/// # Parameters
/// * `env` - Testing environment,
/// * `client` - Name of the client to add.
///
/// # Returns
/// * Pubkey of the client's wallet.
#[must_use]
pub async fn add_client(env: &mut Environment, client: impl Into<String>) -> Pubkey {
    let client = client.into();
    let keypair = keypair_from_seed_phrase_and_passphrase(&client, "passphrase").unwrap();
    let key = keypair.pubkey();
    env.wallets.insert(client.clone(), keypair);
    key
}

pub async fn create_ata(
    env: &mut Environment,
    client: &str,
    symbol: &str,
    is_project: bool,
) -> Pubkey {
    if is_project {
        create_project_ata(env, client, symbol).await
    } else {
        create_stable_ata(env, client, symbol).await
    }
}

async fn create_stable_ata(env: &mut Environment, client: &str, symbol: &str) -> Pubkey {
    let account_name = format!("{} ({})", client, symbol);
    let client_key = if client == "Bangk" {
        env.payer.pubkey()
    } else {
        env.wallets[client].pubkey()
    };

    println!("getting mint for {symbol}");
    let token_key = get_ata_address(&client_key, &env.accounts[symbol].0);
    if env.get_token_amount(token_key).await.is_some() {
        return token_key;
    }

    let mint = StableMint::get_address(symbol).0;

    env.accounts.insert(account_name, (token_key, 0));
    let instr = create_account(&client_key, &mint).unwrap();
    env.execute_transaction(&[instr], &["delegate", client])
        .await;

    token_key
}

async fn create_project_ata(env: &mut Environment, client: &str, symbol: &str) -> Pubkey {
    let account_name = format!("{} ({})", client, symbol);
    let client_key = if client == "Bangk" {
        env.payer.pubkey()
    } else {
        env.wallets[client].pubkey()
    };
    println!("getting mint for {symbol}");
    let token_key = get_ata_address(&client_key, &env.accounts[symbol].0);
    if env.get_token_amount(token_key).await.is_some() {
        return token_key;
    }

    env.accounts.insert(account_name, (token_key, 0));
    let (mint_project, _) = Project::get_address(symbol);
    let instr = create_project_account(&client_key, &mint_project).unwrap();
    env.execute_transaction(&[instr], &["delegate", client])
        .await;

    token_key
}

pub async fn add_exchange(env: &mut Environment, currency: &str, amount: u64) {
    let ata = create_ata(env, "Bangk", currency, false).await;
    let mint = StableMint::get_address(currency).0;

    env.execute_transaction(
        &[mint_stable_coin(&ata, &mint, amount).unwrap()],
        &["delegate"],
    )
    .await;
}

/// Creates a client's ATA for a given currency.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `client` - Client for whom to create the ATA,
/// * `currency` - Currency in the ATA,
/// * `amount` - Initial amount to mint.
///
/// # Returns
/// * Pubkey of the created ATA
#[must_use]
pub async fn mint_stable(
    env: &mut Environment,
    client: &str,
    currency: &str,
    amount: u64,
) -> Pubkey {
    let account_name = format!("{} ({})", client, currency);
    let client_key = env.wallets[client].pubkey();
    let token_key = get_ata_address(&client_key, &env.accounts[currency].0);
    env.accounts.insert(account_name, (token_key, 0));

    // Will create the ATA first if necessary
    let token_key = create_ata(env, client, currency, false).await;
    let mint = StableMint::get_address(currency).0;

    env.execute_transaction(
        &[mint_stable_coin(&token_key, &mint, amount).unwrap()],
        &["delegate"],
    )
    .await;
    token_key
}

pub async fn burn_stable(
    env: &mut Environment,
    client: &str,
    currency: &str,
    amount: u64,
    close_empty: bool,
) {
    let ata = env.accounts[&format!("{client} ({currency})")].0;
    let mint = StableMint::get_address(currency).0;
    env.execute_transaction(
        &[burn_stable_coin(&ata, &mint, amount, close_empty).unwrap()],
        &["delegate"],
    )
    .await;
}

/// Transfer stable coin tokens from one client to another.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `client_source` - Client the coins come from,
/// * `client_target` - Client the coins go to,
/// * `currency` - Currency of the exchanged coins,
/// * `amount` - Number of coins transfered.
pub async fn transfer_stable(
    env: &mut Environment,
    client_source: &str,
    client_target: &str,
    currency: &str,
    amount: u64,
) {
    let ata_source = env.accounts[&format!("{client_source} ({currency})")].0;
    let ata_target = env.accounts[&format!("{client_target} ({currency})")].0;
    let mint = StableMint::get_address(currency).0;
    env.execute_transaction(
        &[transfer_stable_coin(&ata_source, &ata_target, &mint, amount).unwrap()],
        &["delegate"],
    )
    .await;
}

/// Exchange tokens from one currency to another.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `client` - Client performing the exchange,
/// * `source` - Source currency,
/// * `target` - Target currency,
/// * `amount` - Amount of the source currency to exchange,
/// * `exchange_rate` - Exchange rate from source currency to target currency.
pub async fn exchange_stable(
    env: &mut Environment,
    client: &str,
    source: &str,
    target: &str,
    amount: u64,
    exchange_rate: f64,
) {
    let ata_source = env.accounts[&format!("{client} ({source})")].0;
    let ata_target = env.accounts[&format!("{client} ({target})")].0;
    let mint_source = StableMint::get_address(source).0;
    let mint_target = StableMint::get_address(target).0;
    env.execute_transaction(
        &[exchange_stable_coin(
            &ata_source,
            &ata_target,
            &mint_source,
            &mint_target,
            amount,
            exchange_rate,
        )
        .unwrap()],
        &["delegate"],
    )
    .await;
}

/// Create a new project
///
/// # Parameters
/// * `env` - Testing environment,
/// * `id` - Name, symbol and URI of the project,
/// * `currency` - Symbol of the project's preferred currency,
/// * `interest_rate` - Rate of interest of the project (fixed or targetted),
/// * `token_value` - Value of one token in the project's preferred currency,
/// * `payment_periodicity` - [Periodicity](crate::utils::projects::Periodicity) at which the interests are to be paid,
/// * `risk_assessment` - European standard risk assessment (1 to 7),
///
/// # Returns
/// * Pubkey of the project's mint.
pub async fn add_project<T>(
    env: &mut Environment,
    id: (T, T, T),
    currency: &str,
    interest_rate: f64,
    token_value: u32,
    payment_periodicity: Periodicity,
    risk_assessment: u8,
) -> Pubkey
where
    T: Into<String>,
{
    let name = id.0.into();
    let symbol = id.1.into();
    let uri = id.2.into();
    let (mint_key, mint_bump) = Project::get_address(&symbol);
    env.accounts.insert(symbol.clone(), (mint_key, mint_bump));
    let _ = add_client(env, &symbol).await;
    let ata_project = create_ata(env, &symbol, currency, false).await;
    env.accounts
        .insert(format!("{} ({})", symbol, currency), (ata_project, 0));

    println!("project address: {mint_key}");

    env.execute_transaction(
        &[initialize_invest_project(
            (name, symbol, uri),
            &ata_project,
            interest_rate,
            token_value,
            payment_periodicity,
            risk_assessment,
        )
        .unwrap()],
        &["delegate"],
    )
    .await;

    mint_key
}

/// Launches a project.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `mint_project` - Mint associated to the project.
pub async fn launch_project(env: &mut Environment, project_symbol: &str) {
    let (mint_project, _) = Project::get_address(project_symbol);
    env.execute_transaction(
        &[change_project_status(&mint_project, ProjectStatus::Live).unwrap()],
        &["delegate"],
    )
    .await;
}

/// Creates a client's investment to a project.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `project_symbol` - Symbol (code) of the project,
/// * `client_name` - Name of the investing client,
/// * `currency` - The client's preferred currency.
/// * `amount` - Amount of tokens to buy,
/// * `cost` - Cost of the investment,
///
/// # Returns
/// * Address of the ATA,
/// * Address of the record PDA,
/// * Seed bump used to generate the record PDA.
pub async fn create_client_investment(
    env: &mut Environment,
    project_symbol: &str,
    client_name: &str,
    currency: &str,
    amount: u64,
) -> (Pubkey, Pubkey, u8) {
    let client = &env.wallets[client_name].pubkey();
    let (mint_stable, _) = StableMint::get_address(currency);
    let project_stable = env.accounts[format!("{} ({})", project_symbol, currency).as_str()].0;

    // 1. Create the client's project ATA
    let client_token = create_ata(env, client_name, project_symbol, true).await;

    // 2. Get the address for the investment record
    let mint_project = env.accounts[project_symbol].0;
    let (client_record, client_record_bump) = Investment::get_address(&[client, &mint_project]);

    // 3. Get the project's mint
    let (mint_project, _) = Project::get_address(project_symbol);

    // 4. Create the investment
    env.execute_transaction(
        &[
            investment_client(&mint_project, client, &project_stable, &mint_stable, amount)
                .unwrap(),
        ],
        &["delegate"],
    )
    .await;

    // 3. return res
    (client_token, client_record, client_record_bump)
}

/// Creates a client's investment to a project.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `project_symbol` - Symbol (code) of the project,
/// * `client_name` - Name of the investing client,
/// * `currency_project` - Currency used by the project,
/// * `currency_clients` - Currency used by the client,
/// * `amount` - Amount of tokens to buy,
/// * `cost` - Cost of the investment,
/// * `exchange_rate` - Exchange rate from client's currency to project's.
///
/// # Returns
/// * Address of the ATA,
/// * Address of the record PDA,
/// * Seed bump used to generate the record PDA.
pub async fn create_client_investment_exchange(
    env: &mut Environment,
    project_symbol: &str,
    client_name: &str,
    currency_project: &str,
    currency_clients: &str,
    amount: u64,
    exchange_rate: f64,
) -> (Pubkey, Pubkey, u8) {
    let project_stable =
        env.accounts[format!("{} ({})", project_symbol, currency_project).as_str()].0;
    let client = &env.wallets[client_name].pubkey();

    let (mint_stable_project, _) = StableMint::get_address(currency_project);
    let (mint_stable_clients, _) = StableMint::get_address(currency_clients);
    let mint_project = env.accounts[project_symbol].0;
    let (client_record, client_record_bump) = Investment::get_address(&[client, &mint_project]);
    let client_token = create_ata(env, client_name, project_symbol, true).await;

    env.execute_transaction(
        &[investment_client_with_exchange(
            &mint_project,
            client,
            &project_stable,
            &mint_stable_project,
            &mint_stable_clients,
            amount,
            exchange_rate,
        )
        .unwrap()],
        &["delegate"],
    )
    .await;
    (client_token, client_record, client_record_bump)
}

/// Transfer invest tokens from one client to another.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `project_symbol` - Symbol of the project,
/// * `currency` - Currency used by the project and clients,
/// * `seller` - Name of the seller,
/// * `buyer` - Name of the buyer,
/// * `amount` - Number of tokens to transfer,
/// * `cost` - Cost of the transaction.
pub async fn transfer_invest(
    env: &mut Environment,
    project_symbol: &str,
    currency: &str,
    seller: &str,
    buyer: &str,
    amount: u64,
    cost: u64,
) {
    let (mint_stable, _) = StableMint::get_address(currency);
    let (mint_project, _) = Project::get_address(project_symbol);
    env.execute_transaction(
        &[transfer_investment(
            &mint_project,
            &mint_stable,
            &env.wallets[seller].pubkey(),
            &env.wallets[buyer].pubkey(),
            amount,
            cost,
        )
        .unwrap()],
        &["delegate"],
    )
    .await;
}

/// Transfer invest tokens from one client to another.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `project_symbol` - Symbol of the project,
/// * `currency_seller` - Currency used by the seller,
/// * `currency_buyer` - Currency used by the buyer,
/// * `seller` - Name of the seller,
/// * `buyer` - Name of the buyer,
/// * `amount` - Number of tokens to transfer,
/// * `cost` - Cost of the transaction,
/// * `exchange_rate` - Exchange rate from buyer to seller.
#[allow(clippy::too_many_arguments)]
pub async fn transfer_invest_with_exchange(
    env: &mut Environment,
    project_symbol: &str,
    currency_buyer: &str,
    currency_seller: &str,
    seller: &str,
    buyer: &str,
    amount: u64,
    cost: u64,
    exchange_rate: f64,
) {
    let (mint_stable_buyer, _) = StableMint::get_address(currency_buyer);
    let (mint_stable_seller, _) = StableMint::get_address(currency_seller);
    let (mint_project, _) = Project::get_address(project_symbol);

    env.execute_transaction(
        &[transfer_investment_with_exchange(
            &mint_project,
            &mint_stable_buyer,
            &mint_stable_seller,
            &env.wallets[seller].pubkey(),
            &env.wallets[buyer].pubkey(),
            amount,
            cost,
            exchange_rate,
        )
        .unwrap()],
        &["delegate"],
    )
    .await;
}

/// Pay dividends from a project to clients
///
/// # Parameters
/// * `env` - Testing environment,
/// * `project_symbol` - Symbol of the project,
/// * `project_mint` - Mint associated with the project,
/// * `currency` - Currency used by the project and clients,
/// * `nclients` - Number of clients (supposedly) in the batch,
/// * `clients` - List of clients IDs,
/// * `interest_rate` - Interest rate to pay,
pub async fn pay_dividends(
    env: &mut Environment,
    project_symbol: &str,
    currency: &str,
    clients: &[&str],
    interest_rate: f64,
) {
    let project_stable = env.accounts[format!("{} ({})", project_symbol, currency).as_str()].0;
    let (mint_stable, _) = StableMint::get_address(currency);
    let (mint_project, _) = Project::get_address(project_symbol);

    let mut keys = Vec::new();
    clients
        .iter()
        .for_each(|name| keys.push(env.wallets[*name].pubkey()));

    let mut instructions = Vec::new();
    keys.iter().for_each(|key| {
        instructions.push(
            pay_investment_dividends(
                &mint_project,
                &project_stable,
                &mint_stable,
                key,
                interest_rate,
            )
            .unwrap(),
        );
    });
    env.execute_transaction(instructions.as_slice(), &["delegate"])
        .await;
}

/// Pay dividends from a project to clients with a currency exchange.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `project_symbol` - Symbol of the project,
/// * `project_mint` - Mint associated with the project,
/// * `currency_project` - Currency used by the project,
/// * `currency_clients` - Currency used by the clients,
/// * `nclients` - Number of clients (supposedly) in the batch,
/// * `clients` - List of client IDs,
/// * `interest_rate` - Interest rate to pay,
/// * `exchange_rate` - Exchange rate between the project's currency and the clients',
pub async fn pay_dividends_with_exchange(
    env: &mut Environment,
    project_symbol: &str,
    currency_project: &str,
    currency_clients: &str,
    clients: &[&str],
    interest_rate: f64,
    exchange_rate: f64,
) {
    let project_stable =
        env.accounts[format!("{} ({})", project_symbol, currency_project).as_str()].0;
    let (mint_project, _) = Project::get_address(project_symbol);
    let (mint_stable_project, _) = StableMint::get_address(currency_project);
    let (mint_stable_clients, _) = StableMint::get_address(currency_clients);

    let mut keys = Vec::new();
    clients
        .iter()
        .for_each(|name| keys.push(env.wallets[*name].pubkey()));
    let mut instructions = Vec::new();
    keys.iter().for_each(|key| {
        instructions.push(
            pay_investment_dividends_with_exchange(
                &mint_project,
                &project_stable,
                &mint_stable_project,
                &mint_stable_clients,
                key,
                interest_rate,
                exchange_rate,
            )
            .unwrap(),
        );
    });
    env.execute_transaction(instructions.as_slice(), &["delegate"])
        .await;
}

/// Closes or cancels a project.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `action` - Close or cancel,
/// * `project_symbol` - Symbol of the project to end,
/// * `project_mint` - Address of the mint associated with the project,
/// * `currency` - Currency used by the project & clients,
///  * `clients` - List of client IDs.
pub async fn end_project(
    env: &mut Environment,
    action: &str,
    project_symbol: &str,
    currency: &str,
    clients: &[&str],
) {
    let project_stable = env.accounts[format!("{} ({})", project_symbol, currency).as_str()].0;
    let (mint_stable, _) = StableMint::get_address(currency);
    let (mint_project, _) = Project::get_address(project_symbol);
    let instr =
        move |key| reimburse_client(&mint_project, &project_stable, &mint_stable, key).unwrap();

    let mut keys = Vec::new();
    clients
        .iter()
        .for_each(|name| keys.push(env.wallets[*name].pubkey()));
    let mut instructions = Vec::new();
    keys.iter().for_each(|key| instructions.push(instr(key)));

    instructions.push(
        change_project_status(
            &mint_project,
            match action {
                "close" => ProjectStatus::Closed,
                "cancel" => ProjectStatus::Cancelled,
                _ => panic!(
                    "unknown action {} to end project {}",
                    action, project_symbol
                ),
            },
        )
        .unwrap(),
    );

    env.execute_transaction(instructions.as_slice(), &["delegate"])
        .await;
}

/// Closes or cancels a project.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `action` - Close or cancel,
/// * `project_symbol` - Name of the project to end,
/// * `project_mint` - Address of the mint associated with the project,
/// * `currency_project` - Currency used by the project,
/// * `currency_clients` - Currency used by the client,
///  * `clients` - Accounts of the clients.
pub async fn end_project_with_exchange(
    env: &mut Environment,
    action: &str,
    project_symbol: &str,
    currency_project: &str,
    currency_clients: &str,
    clients: &[&str],
    exchange_rate: f64,
) {
    let project_stable =
        env.accounts[format!("{} ({})", project_symbol, currency_project).as_str()].0;

    let (mint_project, _) = Project::get_address(project_symbol);
    let (mint_stable_project, _) = StableMint::get_address(currency_project);
    let (mint_stable_client, _) = StableMint::get_address(currency_clients);
    let instr = move |key| {
        reimburse_client_with_exchange(
            &mint_project,
            &project_stable,
            &mint_stable_project,
            &mint_stable_client,
            &key,
            exchange_rate,
        )
        .unwrap()
    };

    let mut keys = Vec::new();
    clients
        .iter()
        .for_each(|name| keys.push(env.wallets[*name].pubkey()));
    let mut instructions = Vec::new();
    keys.into_iter()
        .for_each(|key| instructions.push(instr(key)));

    instructions.push(
        change_project_status(
            &mint_project,
            match action {
                "close" => ProjectStatus::Closed,
                "cancel" => ProjectStatus::Cancelled,
                _ => panic!(
                    "unknown action {} to end project {}",
                    action, project_symbol
                ),
            },
        )
        .unwrap(),
    );
    env.execute_transaction(instructions.as_slice(), &["delegate"])
        .await;
}

/// Changes the next payment date for a project.
///
/// # Parameters
/// * `env` - Testing environment,
/// * `project_mint` - Mint of the project to update,
/// * `next_payment` - New next payment date.
pub async fn reset_next_payment(
    env: &mut Environment,
    project_mint: &Pubkey,
    next_payment: DateTime<Utc>,
) {
    // Reset the next payment date to the past
    let instruction = update_field(
        &spl_token_2022::id(),
        project_mint,
        &env.delegate.pubkey(),
        Key(String::from("next")),
        format!("{}", next_payment.timestamp() - 10),
    );
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&env.delegate.pubkey()));
    transaction.sign(&[&env.delegate], env.blockhash);
    env.client.process_transaction(transaction).await.unwrap();
}
