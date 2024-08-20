// File: bangk/src/instruction.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/instruction.rs
// Project: bangk-onchain
// Creation date: Friday 08 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::pattern_type_mismatch)] // Shank triggers it?

use bangk_onchain_common::Error;
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

use crate::{
    processor::{BANGK, DELEGATE},
    state::{
        clients::Investment,
        dividends_tracker::DividendsTracker,
        mint_data::MintData,
        pda::BangkPda as _,
        projects::{Periodicity, Project, ProjectBuilder, ProjectStatus},
        stable::StableMint,
    },
    utils::get_ata_address,
};

/// Arguments to Initialize a new Stable Coin.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct CreateStableCoinArgs {
    /// Name of the currency' stable coin to initialize
    pub currency: String,
    /// Symbol of the coin.
    pub symbol: String,
    /// `URI` of the coin.
    pub uri: String,
    /// Number of decimals to use.
    pub decimals: u8,
}

/// A generic structure for when the only argument
/// is a number of tokens.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct TokenAmountArgs {
    /// Amount of tokens to burn, mint or transfer
    pub amount: u64,
}

/// A generic structure for when the only arguments are
/// an amount of tokens and an exchange rate.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct ExchangeStableCoinsArgs {
    /// Amount of tokens to burn, mint or transfer
    pub amount: u64,
    /// Exchange rate (at 1e12).
    pub exchange_rate: u64,
}

/// Argument to burn stable coins.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct BurnStableCoinsArgs {
    /// Amount to burn.
    pub amount: u64,
    /// Determines if an empty account should be closed or kept.
    pub close_empty: bool,
}

/// Arguments to initialize a project.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct CreateInvestProjecArgs {
    /// Project for which to create the new mint,
    pub project: Project,
    /// Seed bump for the dividends tracker PDA.
    pub bump: u8,
}

/// Arguments for a new client investment.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct InvestmentClientArgs {
    /// Seed bump used to derive the client's PDA.
    pub record_bump: u8,
    /// Amount of tokens to buy.
    pub amount: u64,
}

/// Arguments for a new client investing in a project in a
/// different currency than it uses.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct InvestmentClientWithExchangeArgs {
    /// Seed bump used to derive the client's PDA.
    pub record_bump: u8,
    /// Amount of tokens to buy.
    pub amount: u64,
    /// Exchange rate for the foreign currency (at 1e12).
    pub exchange_rate: u64,
}

/// Arguments to transfer project tokens between two clients.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct TransferInvestmentArgs {
    /// Seed bump used to derive the client's PDA.
    pub record_bump: u8,
    /// Number of tokens to transfer.
    pub amount: u64,
    /// Cost of the tokens in stable money.
    pub cost: u64,
}

/// Arguments to transfer project tokens between two clients using different currencies.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct TransferInvestmentWithExchangeArgs {
    /// Seed bump used to derive the client's PDA.
    pub record_bump: u8,
    /// Number of tokens to transfer.
    pub amount: u64,
    /// Cost of the tokens in stable money.
    pub cost: u64,
    /// Exchange rate for the foreign currency (at 1e12).
    pub exchange_rate: u64,
}

/// Arguments for a project to pay dividends.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct PayInvestmentDividendsArgs {
    /// Interest rate to pay (at 1e6).
    pub interest: u32,
}

/// Arguments for a project to pay dividends to clients using a different currency.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct PayInvestmentDividendsWithExchangeArgs {
    /// Interest rate to pay (at 1e6).
    pub interest: u32,
    /// Exchange rate for the foreign currency (at 1e12).
    pub exchange_rate: u64,
}

/// Argument for a number of clients in a batch.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct ChangeProjectStatusArgs {
    /// Number of clients in the batch
    pub status: ProjectStatus,
}

/// Argument for a simple exchange rate argument.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct ExchangeRateArgs {
    /// Exchange rate for the foreign currency (at 1e12).
    pub exchange_rate: u64,
}

/// Global payload for Bangk program.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, ShankInstruction)]
#[rustfmt::skip]
#[repr(u64)]
pub enum BangkInstruction {
    /// Initialize the stable coin Mint.
    ///
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. 󰴒 Mint of the stable coin to create.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="mint", desc="PDA where the new mint will be created")]
    CreateStableCoin(CreateStableCoinArgs),

    /// Create an ATA for a client.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. 󰴹 Owner of the account in which the tokens will be minted,
    /// 3. Stable mint for which the ATA will be create,
    /// 4. 󰴒 ATA to be created,
    /// 5. System program,
    /// 6. SPL 2022 Token program,
    /// 7. Associated Token Account program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, signer, name="client", desc="Wallet of the client for whom the account will be created")]
    #[account(2, writable, name="mint", desc="Mint for which the ATA will be created")]
    #[account(3, writable, name="ata_client", desc="The account to create")]
    #[account(4, name="system_program")]
    #[account(5, name="spl_program_2022", desc="SPL Token 2022 Program")]
    #[account(6, name="ata_program")]
    CreateClientAccount,

    /// Mint stable coins to a client's account.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 3. 󰴒 Stable mint from which to mint tokens,
    /// 4. 󰴒 ATA where the tokens will be minted (will be created if needed),
    /// 5. System program,
    /// 6. SPL 2022 Token program,
    /// 7. Associated Token Account program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="mint", desc="Mint of the newly created stable coins")]
    #[account(2, writable, name="ata_client", desc="The client's ATA for the given currency, will be created if needed")]
    #[account(3, name="system_program")]
    #[account(4, name="spl_program_2022", desc="SPL Token 2022 Program")]
    #[account(5, name="ata_program")]
    MintStableCoin(TokenAmountArgs),

    /// Transfer stable coins from one account to another.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint the tokens to transfer are associated to,
    /// 3. 󰴒 Source ATA,
    /// 4. 󰴒 Target ATA,
    /// 5. SPL 2022 Token program.
    #[account(0, signer, writable, name="signer", desc="The transfer authority, which is either Bangk or the client owning ata_source")]
    #[account(1, name="mint", desc="Mint the transferred stable coins are associated to")]
    #[account(2, name="ata_source", desc="The source ATA")]
    #[account(3, name="ata_target", desc="The target ATA")]
    #[account(4, name="spl_program_2022", desc="SPL Token 2022 Program")]
    TransferStableCoin(TokenAmountArgs),

    /// Exchange stable coins from one currency into another.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Stable mint of the source currency,
    /// 3. 󰴒 Source ATA,
    /// 4. 󰴒 Bangk ATA for the source currency,
    /// 5. Stable mint of the target currency,
    /// 6. 󰴒 Target ATA,
    /// 7. 󰴒 Bangk ATA for the target currency,
    /// 8. SPL 2022 Token program account.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_source", desc="Mint the source stable coins are associated to")]
    #[account(2, writable, name="ata_source", desc="The source ATA")]
    #[account(3, writable, name="exchange_source", desc="The ATA being used as a relay for the source stable coins")]
    #[account(4, name="mint_target", desc="Mint the target stable coins are associated to")]
    #[account(5, writable, name="ata_target", desc="The target ATA")]
    #[account(6, writable, name="exchange_target", desc="The ATA being used as a relay for the target stable coins")]
    #[account(7, name="spl_program_2022", desc="SPL Token 2022 Program")]
    ExchangeStableCoin(ExchangeStableCoinsArgs),

    /// Burn stable coins from a client's account.
    ///
    /// # Accounts
    /// 1. 󰴹 󰴒 Transaction payer,
    /// 2. Mint the tokens to burn belong to,
    /// 3. 󰴒 Associated Token Account owning the tokens to burn,
    /// 4. SPL 2022 Token program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint", desc="Mint of the burnt stable coins")]
    #[account(2, writable, name="ata_client", desc="The client's ATA for the given currency, might be destroyed if empty")]
    #[account(3, name="spl_program_2022", desc="SPL Token 2022 Program")]
    BurnStableCoin(BurnStableCoinsArgs),

    /// Create a new project.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. 󰴒 Mint of the project (will be created),
    /// 3. 󰴒 PDA for the dividends tracker,
    /// 4. System Program,
    /// 5. SPL 2022 Token Program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="mint", desc="PDA where the mint for the project will be created")]
    #[account(2, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(3, name="system_program")]
    #[account(4, name="spl_program_2022", desc="SPL Token 2022 Program")]
    CreateInvestProject(CreateInvestProjecArgs),

    /// A client invests into a new project.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint of the stable currency associated to the project,
    /// 3. 󰴒 ATA of the stable currency for the client,
    /// 4. 󰴒 ATA of the stable currency for the project,
    /// 5. 󰴒 Mint of the project's tokens,
    /// 6. 󰴒 PDA for the dividends tracker,
    /// 7. 󰴒 ATA of the project's tokens for the client,
    /// 8. 󰴒 PDA of the client's investment record for the project,
    /// 9. System program,
    /// 10. SPL 2022 Token program,
    /// 11. ATA program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable", desc="Mint of the stable coins used for payment")]
    #[account(2, writable, name="ata_stable_client", desc="The client't ATA of stable coins")]
    #[account(3, writable, name="ata_stable_project", desc="The project's ATA of stable coins")]
    #[account(4, writable, name="mint_project", desc="Mint of the project's tokens")]
    #[account(5, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(6, writable, name="ata_project_client", desc="The client's ATA of project tokens")]
    #[account(7, writable, name="investment_record", desc="The PDA for the client's investment")]
    #[account(8, name="system_program")]
    #[account(9, name="spl_program_2022", desc="SPL Token 2022 Program")]
    #[account(10, name="ata_program")]
    InvestmentClient(InvestmentClientArgs),

    /// A client's investment into a new project made in a foreign currency.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint of the source currency (the client's),
    /// 3. 󰴒 ATA of the stable currency for the client,
    /// 4. 󰴒 Bangk's ATA for the source currency,
    /// 5. 󰴒 Mint of the target currency (the project's),
    /// 6. 󰴒 ATA of the stable currency for the project,
    /// 7. 󰴒 Bangk's ATA for the target currency,
    /// 8. 󰴒 Mint of the project's tokens,
    /// 9. 󰴒 PDA for the dividends tracker,
    /// 10. 󰴒 ATA of the project's tokens for the client,
    /// 11. 󰴒 PDA of the client's investment record for the project,
    /// 12. System program,
    /// 13. SPL 2022 Token program,
    /// 14. ATA program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable_source", desc="Mint of the stable coins used for payment by the client")]
    #[account(2, writable, name="ata_stable_client", desc="The client't ATA of stable coins")]
    #[account(3, writable, name="exchange_source", desc="The ATA being used as a relay for the source stable coins")]
    #[account(4, name="mint_stable_target", desc="Mint of the stable coins used by the project to receive the payment")]
    #[account(5, writable, name="ata_stable_project", desc="The project's ATA of stable coins")]
    #[account(6, writable, name="exchange_target", desc="The ATA being used as a relay for the target stable coins")]
    #[account(7, writable, name="mint_project", desc="Mint of the project's tokens")]
    #[account(8, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(9, writable, name="ata_project_client", desc="The client's ATA of project tokens")]
    #[account(10, writable, name="investment_record", desc="The PDA for the client's investment")]
    #[account(11, name="system_program")]
    #[account(12, name="spl_program_2022", desc="SPL Token 2022 Program")]
    #[account(13, name="ata_program")]
    InvestmentClientWithExchange(InvestmentClientWithExchangeArgs),

    /// Transfer a project's tokens from one client to another.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint of the stable currency,
    /// 3. 󰴒 ATA of the buyer's stable coins,
    /// 4. 󰴒 ATA of the seller's stable coins,
    /// 5. Mint of the project's tokens,
    /// 6. 󰴒 PDA for the dividends tracker,
    /// 7. 󰴒 ATA of the buyer's project tokens (created if necessary),
    /// 8. 󰴒 ATA of the seller's project tokens (destroyed if necessary),
    /// 9. 󰴒 PDA of the buyer's investment record (created if necessary),
    /// 10. 󰴒 PDA of the seller's investment record (destroyed if necessary),
    /// 11. System program,
    /// 12. SPL 2022 Token Program,
    /// 13. ATA Program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable", desc="Mint of the stable coins used for payment")]
    #[account(2, writable, name="ata_stable_buyer", desc="The buyer't ATA of stable coins")]
    #[account(3, writable, name="ata_stable_seller", desc="The seller't ATA of stable coins")]
    #[account(4, name="mint_project", desc="Mint of the project's tokens")]
    #[account(5, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(6, writable, name="ata_project_buyer", desc="The buyer's ATA of project tokens")]
    #[account(7, writable, name="ata_project_seller", desc="The seller's ATA of project tokens")]
    #[account(8, writable, name="investment_record_buyer", desc="The PDA for the buyer's investment")]
    #[account(9, writable, name="investment_record_seller", desc="The PDA for the seller's investment")]
    #[account(10, name="system_program")]
    #[account(11, name="spl_program_2022", desc="SPL Token 2022 Program")]
    #[account(12, name="ata_program")]
    TransferInvestment(TransferInvestmentArgs),

    /// Transfer a project's tokens from one client to another in a different currency.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint of the source currency (the buyer's),
    /// 3. 󰴒 ATA of the buyer's stable coins,
    /// 4. 󰴒 ATA of Bangk's source currency,
    /// 5. Mint of the target currency (the seller's),
    /// 6. 󰴒 ATA of the seller's stable coins,
    /// 7. 󰴒 ATA of Bangk's target currency,
    /// 8. 󰴒 Mint of the project's tokens,
    /// 9. 󰴒 PDA for the dividends tracker,
    /// 10. 󰴒 ATA of the buyer's project tokens (created if necessary),
    /// 11. 󰴒 ATA of the seller's project tokens (destroyed if necessary),
    /// 12. 󰴒 PDA of the buyer's investment record (created if necessary),
    /// 13. 󰴒 PDA of the seller's investment record (destroyed if necessary),
    /// 14. System program,
    /// 15. SPL 2022 Token Program,
    /// 16. ATA Program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable_source", desc="Mint of the stable coins used for payment")]
    #[account(2, writable, name="ata_stable_buyer", desc="The buyer't ATA of stable coins")]
    #[account(3, writable, name="exchange_source", desc="The ATA being used as a relay for the source stable coins")]
    #[account(4, name="mint_stable_target", desc="Mint of the stable coins the payment is received in")]
    #[account(5, writable, name="ata_stable_seller", desc="The seller't ATA of stable coins")]
    #[account(6, writable, name="exchange_target", desc="The ATA being used as a relay for the target stable coins")]
    #[account(7, name="mint_project", desc="Mint of the project's tokens")]
    #[account(8, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(9, writable, name="ata_project_buyer", desc="The buyer's ATA of project tokens")]
    #[account(10, writable, name="ata_project_seller", desc="The seller's ATA of project tokens")]
    #[account(11, writable, name="investment_record_buyer", desc="The PDA for the buyer's investment")]
    #[account(12, writable, name="investment_record_seller", desc="The PDA for the seller's investment")]
    #[account(13, name="system_program")]
    #[account(14, name="spl_program_2022", desc="SPL Token 2022 Program")]
    #[account(15, name="ata_program")]
    TransferInvestmentWithExchange(TransferInvestmentWithExchangeArgs),

    /// Pay the interests for a project.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint of the project's stable currency,
    /// 3. 󰴒 ATA of the project's stable coin (from which the payment is taken),
    /// 4. 󰴒 Mint of the project's tokens,
    /// 5. 󰴒 PDA for the dividends tracker,
    /// 6. 󰴒 ATA of the client's stable coin (must match the project's, otherwise see [`PayInterestExchange`](BangkInstruction::PayInvestmentDividendsWithExchange)),
    /// 7. 󰴒 ATA of the client's projects tokens,
    /// 8. 󰴒 PDA of the client's [Investment] record.
    /// 9. SPL 2022 Token Program Account.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable", desc="Mint of the stable coins used for payments")]
    #[account(2, writable, name="ata_stable_project", desc="Stable coins ATA for the project")]
    #[account(3, writable, name="mint_project", desc="Mint of the project's tokens")]
    #[account(4, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(5, writable, name="ata_stable_client", desc="Stable coins ATA for the first client (repeating pattern)")]
    #[account(6, writable, name="ata_stable_project", desc="Project tokens ATA for the first client (repeating pattern)")]
    #[account(7, writable, name="investment_record", desc="Investment record for the firt client (repeating pattern)")]
    #[account(8, name="spl_program_2022", desc="SPL Token 2022 Program")]
    PayInvestmentDividends(PayInvestmentDividendsArgs),

    /// Pay the interests for a project.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint of the source currency (the project's),
    /// 3. 󰴒 ATA of the project's stable coin (from which the payment is taken),
    /// 4. 󰴒 Bangk's ATA for the source currency,
    /// 5. Mint of the target currency (the clients'),
    /// 6. 󰴒 Bangk's ATA for the target currency,
    /// 7. 󰴒 Mint of the project's tokens,
    /// 8. 󰴒 PDA for the dividends tracker,
    /// 9. 󰴒 ATA of the client's stable coin (must be different from the project's otherwise see [`PayInterest`](BangkInstruction::PayInvestmentDividends)),
    /// 10. 󰴒 ATA of the client's projects tokens,
    /// 11. 󰴒 PDA of the client's [`Investment`] record.
    /// 12. SPL 2022 Token Program,
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable_source", desc="Mint of the stable coins used for payments")]
    #[account(2, writable, name="ata_stable_project", desc="Stable coins ATA for the project")]
    #[account(3, writable, name="exchange_project", desc="The ATA being used as a relay for the source stable coins")]
    #[account(4, name="mint_stable_target", desc="Mint of the stable coins the payments are received in")]
    #[account(5, writable, name="exchange_target", desc="The ATA being used as a relay for the target stable coins")]
    #[account(6, writable, name="mint_project", desc="Mint of the project's tokens")]
    #[account(7, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(8, writable, name="ata_stable_client", desc="Stable coins ATA for the first client (repeating pattern)")]
    #[account(9, writable, name="ata_stable_project", desc="Project tokens ATA for the first client (repeating pattern)")]
    #[account(10, writable, name="investment_record", desc="Investment record for the firt client (repeating pattern)")]
    #[account(11, name="spl_program_2022", desc="SPL Token 2022 Program")]
    PayInvestmentDividendsWithExchange(PayInvestmentDividendsWithExchangeArgs),

    /// Change the status of a project.
    ///
    /// # Accounts
    /// 1. 󰴹 󰴒 Transaction payer,
    /// 2. 󰴒 Mint of the project,
    /// 3. 󰴒 PDA for the dividends tracker,
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, writable, name="mint_project", desc="Mint of the project's tokens")]
    #[account(2, writable, name="tracker", desc="PDA of the dividends tracker")]
    ChangeProjectStatus(ChangeProjectStatusArgs),

    /// Cancel the project.
    ///
    /// # Accounts
    /// 1. 󰴹 󰴒 Transaction payer,
    /// 2. Mint of the project's (and the client's) currency,
    /// 3. 󰴒 ATA for the project's stable coins,
    /// 4. 󰴒 Mint of the project,
    /// 5. 󰴒 PDA for the dividends tracker,
    /// 6. 󰴒 ATA of the client's stable coin (must match the project's, otherwise see [`ReimburseInvestProjectWithExchange`](BangkInstruction::ReimburseInvestProjectWithExchange)),
    /// 7. 󰴒 ATA of the client's projects tokens (will be destroyed),
    /// 8. 󰴒 PDA of the client's [`Investment`] record (will be destroyed).
    /// 9. SPL 2022 Token Program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable", desc="Mint of the stable coins used for payments")]
    #[account(2, writable, name="ata_stable_project", desc="Stable coins ATA for the project")]
    #[account(3, writable, name="mint_project", desc="Mint of the project's tokens")]
    #[account(4, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(5, writable, name="ata_stable_client", desc="Stable coins ATA for the first client (repeating pattern)")]
    #[account(6, writable, name="ata_stable_project", desc="Project tokens ATA for the first client (repeating pattern)")]
    #[account(7, writable, name="investment_record", desc="Investment record for the firt client (repeating pattern)")]
    #[account(8, name="spl_program_2022", desc="SPL Token 2022 Program")]
    ReimburseInvestProject,

    /// Cancel the project with exchange.
    ///
    /// # Accounts
    /// 1. 󰴹 Transaction payer,
    /// 2. Mint of the source currency (the project's),
    /// 3. 󰴒 ATA for the project's stable coins,
    /// 4. 󰴒 Bangk Exchange in source currency (the project's),
    /// 5. Mint of the target currency (the client's),
    /// 6. 󰴒 Bangk Exchange in target currency (the client's),
    /// 7. 󰴒 Mint of the project,
    /// 8. 󰴒 PDA for the dividends tracker,
    /// 9. 󰴒 ATA of the client's stable coin (must be different from the project's, otherwise see [`ReimburseInvestProject`](BangkInstruction::ReimburseInvestProject)),
    /// 10. 󰴒 ATA of the client's projects tokens (will be destroyed),
    /// 11. 󰴒 PDA of the client's [`Investment`] record (will be destroyed).
    /// 12. SPL 2022 Token Program.
    #[account(0, signer, writable, name="bangk", desc="Bangk signing account")]
    #[account(1, name="mint_stable_source", desc="Mint of the stable coins used for payments")]
    #[account(2, writable, name="ata_stable_project", desc="Stable coins ATA for the project")]
    #[account(3, writable, name="exchange_project", desc="The ATA being used as a relay for the source stable coins")]
    #[account(4, name="mint_stable_target", desc="Mint of the stable coins the payments are received in")]
    #[account(5, writable, name="exchange_target", desc="The ATA being used as a relay for the target stable coins")]
    #[account(6, writable, name="mint_project", desc="Mint of the project's tokens")]
    #[account(7, writable, name="tracker", desc="PDA of the dividends tracker")]
    #[account(8, writable, name="ata_stable_client", desc="Stable coins ATA for the first client (repeating pattern)")]
    #[account(9, writable, name="ata_stable_project", desc="Project tokens ATA for the first client (repeating pattern)")]
    #[account(10, writable, name="investment_record", desc="Investment record for the firt client (repeating pattern)")]
    #[account(11, name="spl_program_2022", desc="SPL Token 2022 Program")]
    ReimburseInvestProjectWithExchange(ExchangeRateArgs),
}

/// Get the instruction to initialize a new stable coin.
///
/// # Parameters
/// * `name` - Name of the new stable coin,
/// * `symbol` - Three letters (more or less) symbol for the new coin,
/// * `uri` - `URI` for the new coin.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn create_stable_coin<T, S, R>(
    name: T,
    symbol: S,
    uri: R,
    decimals: u8,
) -> Result<Instruction, ProgramError>
where
    T: Into<String>,
    S: Into<String>,
    R: Into<String>,
{
    let symbol = symbol.into();
    let (mint, _) = StableMint::get_address(&symbol);
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(BANGK, true),
            AccountMeta::new_readonly(DELEGATE, true),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::CreateStableCoin(CreateStableCoinArgs {
            currency: name.into(),
            symbol,
            uri: uri.into(),
            decimals,
        }))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Create a new ATA account for a client.
///
/// # Parameters
/// * `client` - Client for whom to create the account,
/// * `mint` - Mint for which to create the account,
///
/// # Errors
/// Never.
pub fn create_account(client: &Pubkey, mint: &Pubkey) -> Result<Instruction, ProgramError> {
    let ata = get_ata_address(client, mint);
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new(*client, true),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::CreateClientAccount)
            .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Mint stable tokens to a given user.
///
/// The ATA where the tokens will be minted as well as the
/// public key of the mint will be automatically computed
/// from the given user public key and the currency symbol
/// in order to prevent mistakes.
///
/// # Parameters
/// * `ata` - ATA in which the stable coin will be minted,
/// * `mint` - Mint of the stable coin,
/// * `amount` - Amount to mint.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn mint_stable_coin(
    ata: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new_readonly(DELEGATE, true),
            AccountMeta::new(*mint, false),
            AccountMeta::new(*ata, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::MintStableCoin(TokenAmountArgs {
            amount,
        }))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Burn stable tokens from a given user.
///
/// # Parameters
/// * `ata` - ATA from which the tokens will be burned,
/// * `mint` - Mint of the stable coin,
/// * `amount` - Amount to burn,
/// * `close_empty` - Determines if an empty account should be closed.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn burn_stable_coin(
    ata: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    close_empty: bool,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new(*mint, false),
            AccountMeta::new(*ata, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::BurnStableCoin(BurnStableCoinsArgs {
            amount,
            close_empty,
        }))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Transfer stable coins from one client to another.
///
/// # Parameters
/// * `ata_source` - Client's ATA the coins come from,
/// * `ata_target` - Client's ATA the coins go to,
/// * `mint` - Mint of the stable coin,
/// * `amount` - Number of coins transferred.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn transfer_stable_coin(
    ata_source: &Pubkey,
    ata_target: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new_readonly(DELEGATE, true),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(*ata_source, false),
            AccountMeta::new(*ata_target, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::TransferStableCoin(TokenAmountArgs {
            amount,
        }))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Exchange stable coins from one currency to another.
///
/// This can only be triggered by Bangk due to the exchange rate being given with the instruction.
///
/// # Parameters
/// * `ata_source` - Client's ATA performing the exchange,
/// * `ata_target` - Client's ATA benefiting from the exchange if different,
/// * `mint_source` - Source currency,
/// * `mint_target` - Target currency,
/// * `amount` - Amount of the source currency to exchange,
/// * `exchange_rate` - Exchange rate from source currency to target currency.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn exchange_stable_coin(
    ata_source: &Pubkey,
    ata_target: &Pubkey,
    mint_source: &Pubkey,
    mint_target: &Pubkey,
    amount: u64,
    exchange_rate: f64,
) -> Result<Instruction, ProgramError> {
    let exchange_source = get_ata_address(&BANGK, mint_source);
    let exchange_target = get_ata_address(&BANGK, mint_target);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new_readonly(DELEGATE, true),
            AccountMeta::new_readonly(*mint_source, false),
            AccountMeta::new(*ata_source, false),
            AccountMeta::new(exchange_source, false),
            AccountMeta::new_readonly(*mint_target, false),
            AccountMeta::new(*ata_target, false),
            AccountMeta::new(exchange_target, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::ExchangeStableCoin(
            ExchangeStableCoinsArgs {
                amount,
                exchange_rate: (exchange_rate * 1e12_f64).trunc() as u64,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Create a new investment project.
///
/// # Parameters
///
/// * `id` - Name, symbol and `URI` of the project,
/// * `ata` - Project's stable ATA,
/// * `interest_rate` - Rate of interest of the project (fixed or targeted),
/// * `token_value` - Value of one token in the project's preferred currency,
/// * `payment_periodicity` - Periodicity at which the interests are to be paid,
/// * `risk_assessment` - European standard risk assessment (1 to 7),
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn initialize_invest_project<T, S, R>(
    id: (T, S, R),
    project_ata: &Pubkey,
    interest_rate: f64,
    token_value: u32,
    payment_periodicity: Periodicity,
    risk_assessment: u8,
) -> Result<Instruction, ProgramError>
where
    T: Into<String>,
    S: Into<String>,
    R: Into<String>,
{
    let symbol: String = id.1.into();
    let (mint_project, pda_bump) = Project::get_address(&symbol);
    let (dividends_tracker, tracker_bump) = DividendsTracker::get_address(&[&mint_project]);
    let project = ProjectBuilder::new()
        .id(id.0, symbol, id.2)?
        .seep_bump(pda_bump)
        .ata(project_ata)
        .interest_rate((interest_rate * 1e6_f64).trunc() as u32)?
        .token_value(token_value)?
        .payment_periodicity(payment_periodicity)
        .risk_assessment(risk_assessment)?
        .build();

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(BANGK, true),
            AccountMeta::new_readonly(DELEGATE, true),
            AccountMeta::new(mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::CreateInvestProject(
            CreateInvestProjecArgs {
                project,
                bump: tracker_bump,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Change the status of a project.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `status` - New status of the project.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn change_project_status(
    mint_project: &Pubkey,
    status: ProjectStatus,
) -> Result<Instruction, ProgramError> {
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new_readonly(DELEGATE, true),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::ChangeProjectStatus(
            ChangeProjectStatusArgs { status },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Create a new ATA account for a client.
///
/// # Parameters
/// * `client` - Client for whom to create the account,
/// * `mint_project` - Mint for the project's tokens.
///
/// # Errors
/// Never.
pub fn create_project_account(
    client: &Pubkey,
    mint_project: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let ata = get_ata_address(client, mint_project);
    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*client, true),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::CreateClientAccount)
            .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Creates a client's investment to a project.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `client` - The investing client,
/// * `ata_stable_project` - ATA where the money will be sent,
/// * `mint_stable` - The mint of the stable currency used by the project (and client).
/// * `amount` - Amount of tokens to buy,
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn investment_client(
    mint_project: &Pubkey,
    client: &Pubkey,
    ata_stable_project: &Pubkey,
    mint_stable: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let ata_stable_client = get_ata_address(client, mint_stable);
    let ata_project_client = get_ata_address(client, mint_project);
    let (record_client, record_client_bump) = Investment::get_address(&[client, mint_project]);
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable, false),
            AccountMeta::new(ata_stable_client, false),
            AccountMeta::new(*ata_stable_project, false),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(ata_project_client, false),
            AccountMeta::new(record_client, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::InvestmentClient(InvestmentClientArgs {
            record_bump: record_client_bump,
            amount,
        }))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Creates a client's investment to a project in a different currency.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `client` - The investing client,
/// * `ata_stable_project` - ATA where the money will be sent,
/// * `mint_stable_project` - The mint of the stable currency used by the project.
/// * `mint_stable_client` - The mint of the stable currency used by the client.
/// * `amount` - Amount of tokens to buy,
/// * `exchange_rate` - Exchange rate from client's currency to project's.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn investment_client_with_exchange(
    mint_project: &Pubkey,
    client: &Pubkey,
    ata_stable_project: &Pubkey,
    mint_stable_project: &Pubkey,
    mint_stable_client: &Pubkey,
    amount: u64,
    exchange_rate: f64,
) -> Result<Instruction, ProgramError> {
    let ata_stable_client = get_ata_address(client, mint_stable_client);
    let exchange_client = get_ata_address(&BANGK, mint_stable_client);
    let exchange_project = get_ata_address(&BANGK, mint_stable_project);
    let ata_project_client = get_ata_address(client, mint_project);
    let (record_client, record_client_bump) = Investment::get_address(&[client, mint_project]);
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable_client, false),
            AccountMeta::new(ata_stable_client, false),
            AccountMeta::new(exchange_client, false),
            AccountMeta::new(*mint_stable_project, false),
            AccountMeta::new(*ata_stable_project, false),
            AccountMeta::new(exchange_project, false),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(ata_project_client, false),
            AccountMeta::new(record_client, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::InvestmentClientWithExchange(
            InvestmentClientWithExchangeArgs {
                record_bump: record_client_bump,
                amount,
                exchange_rate: (exchange_rate * 1e12_f64).trunc() as u64,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Transfer invest tokens from one client to another.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `currency` - Currency used by the project and clients,
/// * `seller` - ID of the seller,
/// * `buyer` - ID of the buyer,
/// * `amount` - Number of tokens to transfer,
/// * `cost` - Cost of the transaction.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn transfer_investment(
    mint_project: &Pubkey,
    mint_stable: &Pubkey,
    seller: &Pubkey,
    buyer: &Pubkey,
    amount: u64,
    cost: u64,
) -> Result<Instruction, ProgramError> {
    let ata_buyer_stable = get_ata_address(buyer, mint_stable);
    let ata_seller_stable = get_ata_address(seller, mint_stable);
    let ata_project_buyer = get_ata_address(buyer, mint_project);
    let ata_project_seller = get_ata_address(seller, mint_project);
    let (record_buyer, record_buyer_bump) = Investment::get_address(&[buyer, mint_project]);
    let (record_seller, _) = Investment::get_address(&[seller, mint_project]);
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable, false),
            AccountMeta::new(ata_buyer_stable, false),
            AccountMeta::new(ata_seller_stable, false),
            AccountMeta::new_readonly(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(ata_project_buyer, false),
            AccountMeta::new(ata_project_seller, false),
            AccountMeta::new(record_buyer, false),
            AccountMeta::new(record_seller, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::TransferInvestment(
            TransferInvestmentArgs {
                record_bump: record_buyer_bump,
                amount,
                cost,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Transfer invest tokens from one client to another.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `mint_stable_buyer` - The mint of the stable currency used by the buyer.
/// * `mint_stable_seller` - The mint of the stable currency used by the seller.
/// * `buyer` - ID of the buyer,
/// * `amount` - Number of tokens to transfer,
/// * `cost` - Cost of the transaction.
/// * `exchange_rate` - Exchange rate from buyer to seller.
#[allow(clippy::too_many_arguments)]
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn transfer_investment_with_exchange(
    mint_project: &Pubkey,
    mint_stable_buyer: &Pubkey,
    mint_stable_seller: &Pubkey,
    seller: &Pubkey,
    buyer: &Pubkey,
    amount: u64,
    cost: u64,
    exchange_rate: f64,
) -> Result<Instruction, ProgramError> {
    let ata_buyer_stable = get_ata_address(buyer, mint_stable_buyer);
    let ata_seller_stable = get_ata_address(seller, mint_stable_seller);
    let exchange_buyer = get_ata_address(&BANGK, mint_stable_buyer);
    let exchange_seller = get_ata_address(&BANGK, mint_stable_seller);
    let ata_project_buyer = get_ata_address(buyer, mint_project);
    let ata_project_seller = get_ata_address(seller, mint_project);
    let (record_buyer, record_buyer_bump) = Investment::get_address(&[buyer, mint_project]);
    let (record_seller, _) = Investment::get_address(&[seller, mint_project]);
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable_buyer, false),
            AccountMeta::new(ata_buyer_stable, false),
            AccountMeta::new(exchange_buyer, false),
            AccountMeta::new_readonly(*mint_stable_seller, false),
            AccountMeta::new(ata_seller_stable, false),
            AccountMeta::new(exchange_seller, false),
            AccountMeta::new_readonly(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(ata_project_buyer, false),
            AccountMeta::new(ata_project_seller, false),
            AccountMeta::new(record_buyer, false),
            AccountMeta::new(record_seller, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::TransferInvestmentWithExchange(
            TransferInvestmentWithExchangeArgs {
                record_bump: record_buyer_bump,
                amount,
                cost,
                exchange_rate: (exchange_rate * 1e12_f64).trunc() as u64,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Pay dividends from a project to clients
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `ata_stable_project` - ATA where the money will be taken from,
/// * `currency` - Currency used by the project and clients,
/// * `client` - ID of the client to whom the dividends are paid,
/// * `interest_rate` - Interest rate to pay,
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn pay_investment_dividends(
    mint_project: &Pubkey,
    ata_stable_project: &Pubkey,
    mint_stable: &Pubkey,
    client: &Pubkey,
    interest_rate: f64,
) -> Result<Instruction, ProgramError> {
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable, false),
            AccountMeta::new(*ata_stable_project, false),
            AccountMeta::new(get_ata_address(client, mint_stable), false),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(get_ata_address(client, mint_project), false),
            AccountMeta::new(Investment::get_address(&[client, mint_project]).0, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::PayInvestmentDividends(
            PayInvestmentDividendsArgs {
                interest: (interest_rate * 1e6_f64).trunc() as u32,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Pay dividends from a project to clients with a currency exchange.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `ata_stable_project` - ATA where the money will be taken from,
/// * `mint_stable_project` - The mint of the stable currency used by the project.
/// * `mint_stable_client` - The mint of the stable currency used by the client.
/// * `client` - ID of the client to whom the dividends are paid,
/// * `interest_rate` - Interest rate to pay,
/// * `exchange_rate` - Exchange rate between the project's currency and the clients,
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn pay_investment_dividends_with_exchange(
    mint_project: &Pubkey,
    ata_stable_project: &Pubkey,
    mint_stable_project: &Pubkey,
    mint_stable_client: &Pubkey,
    client: &Pubkey,
    interest_rate: f64,
    exchange_rate: f64,
) -> Result<Instruction, ProgramError> {
    let exchange_project = get_ata_address(&BANGK, mint_stable_project);
    let exchange_clients = get_ata_address(&BANGK, mint_stable_client);
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable_project, false),
            AccountMeta::new(*ata_stable_project, false),
            AccountMeta::new(get_ata_address(client, mint_stable_client), false),
            AccountMeta::new(exchange_project, false),
            AccountMeta::new_readonly(*mint_stable_client, false),
            AccountMeta::new(exchange_clients, false),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(get_ata_address(client, mint_project), false),
            AccountMeta::new(Investment::get_address(&[client, mint_project]).0, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::PayInvestmentDividendsWithExchange(
            PayInvestmentDividendsWithExchangeArgs {
                interest: (interest_rate * 1e6_f64).trunc() as u32,
                exchange_rate: (exchange_rate * 1e12_f64).trunc() as u64,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Closes an investment project.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `ata_stable_project` - ATA where the money will be taken from,
/// * `currency` - Currency used by the project & clients,
///  * `client` - The client to reimburse.
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn reimburse_client(
    mint_project: &Pubkey,
    ata_stable_project: &Pubkey,
    mint_stable: &Pubkey,
    client: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable, false),
            AccountMeta::new(*ata_stable_project, false),
            AccountMeta::new(get_ata_address(client, mint_stable), false),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(get_ata_address(client, mint_project), false),
            AccountMeta::new(Investment::get_address(&[client, mint_project]).0, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::ReimburseInvestProject)
            .map_err(|_| Error::InvalidRawData)?,
    })
}

/// Closes an investment project.
///
/// # Parameters
/// * `mint_project` - Mint for the project's tokens.
/// * `ata_stable_project` - ATA where the money will be taken from,
/// * `mint_stable_project` - The mint of the stable currency used by the project.
/// * `mint_stable_client` - The mint of the stable currency used by the client.
///  * `client` - The client to reimburse.
/// * `exchange_rate` - Exchange rate between the project's currency and the client,
///
/// # Errors
/// If instruction's data could not be serialized (so…never?)
pub fn reimburse_client_with_exchange(
    mint_project: &Pubkey,
    ata_stable_project: &Pubkey,
    mint_stable_project: &Pubkey,
    mint_stable_client: &Pubkey,
    client: &Pubkey,
    exchange_rate: f64,
) -> Result<Instruction, ProgramError> {
    let exchange_project = get_ata_address(&BANGK, mint_stable_project);
    let exchange_clients = get_ata_address(&BANGK, mint_stable_client);
    let (dividends_tracker, _) = DividendsTracker::get_address(&[mint_project]);

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(DELEGATE, true),
            AccountMeta::new_readonly(*mint_stable_project, false),
            AccountMeta::new(*ata_stable_project, false),
            AccountMeta::new(get_ata_address(client, mint_stable_client), false),
            AccountMeta::new(exchange_project, false),
            AccountMeta::new_readonly(*mint_stable_client, false),
            AccountMeta::new(exchange_clients, false),
            AccountMeta::new(*mint_project, false),
            AccountMeta::new(dividends_tracker, false),
            AccountMeta::new(get_ata_address(client, mint_project), false),
            AccountMeta::new(Investment::get_address(&[client, mint_project]).0, false),
            AccountMeta::new_readonly(spl_token_2022::id(), false),
        ],
        data: borsh::to_vec(&BangkInstruction::ReimburseInvestProjectWithExchange(
            ExchangeRateArgs {
                exchange_rate: (exchange_rate * 1e12_f64).trunc() as u64,
            },
        ))
        .map_err(|_| Error::InvalidRawData)?,
    })
}
