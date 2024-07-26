// File: bangk/src/state/projects.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/projects.rs
// Project: bangk-onchain
// Creation date: Thursday 23 November 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use core::fmt;
use std::fmt::Display;

use bangk_onchain_common::{debug, Error};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Days, Months, Utc};
use shank::ShankAccount;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_token_metadata_interface::state::TokenMetadata;

use super::{mint_data::MintData, token_metadata};

/// Different status a project can have.
#[derive(
    BorshDeserialize, BorshSerialize, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum ProjectStatus {
    /// Open: the project has been created, clients can invest
    #[default]
    Open,
    /// Live: the project has been launched, clients can trade tokens, interest are paid regularly
    Live,
    /// Closed: the project is finished after being launched, no operation can happen
    Closed,
    /// Canceled: the project was not launched, no operation can happen
    Cancelled,
}

impl Display for ProjectStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = match self {
            Self::Open => "Open",
            Self::Live => "Live",
            Self::Closed => "Closed",
            Self::Cancelled => "Cancelled",
        };
        write!(formatter, "{val}")
    }
}

/// Payment periodicity of the interests.
///
/// * 0: Daily
/// * 1: Weekly
/// * 2: Monthly
/// * 3: Quarterly
/// * 4: Twice a year
/// * 5: Annually
#[derive(BorshDeserialize, BorshSerialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Periodicity {
    /// Payment happens every 24 hours.
    Daily,
    /// Payment happens every week.
    Weekly,
    /// Payment happens every month.
    Monthly,
    /// Payment happens every 3 months.
    Quarterly,
    /// Payment happens every 6 months.
    BiAnnually,
    /// Payment happens every year.
    #[default]
    Annually,
}

/// Data contained in a project account.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Default, ShankAccount, PartialEq, Eq)]
pub struct Project {
    /// ID of the project.
    pub name: String,
    /// Symbol of the associated token.
    pub symbol: String,
    /// `URI` describing the project.
    pub uri: String,
    /// Seed bump used to derive the mint's PDA.
    pub seed_bump: u8,
    /// Stable ATA account associated with the project (which also defines the preferred currency).
    pub ata: String,
    /// Interest rate paid to the investors (as 1e6).
    pub interest_rate: u32,
    /// Value in the preferred Stable Coin of one token for this project.
    pub token_value: u32,
    /// [Periodicity] for the payment of the interests.
    pub payment_periodicity: Periodicity,
    /// Standardized risk assessment (1 => Low risk, 7 => High risk).
    pub risk_assessment: u8,
    /// Timestamp of the last payment. If 0, no payment was made.
    pub last_payment: i64,
    /// Timestamp of the next payment. If 0, no payment is planned.
    pub next_payment: i64,
    /// Current status of the project.
    pub status: ProjectStatus,
    /// Minimum amount to raise to launch the project.
    pub min_goal: u32,
    /// Maximum amount to raise before launching the project.
    pub max_goal: u32,
}

impl Project {
    /// Changes the status of a project from Open to Live.
    ///
    /// # Errors
    /// If the next payment date could not be properly computed.
    ///
    /// Example
    /// ```
    /// # use bangk::state::projects::Project;
    /// # use bangk::state::projects::ProjectBuilder;
    /// # use bangk::state::projects::Periodicity;
    /// # use solana_program::pubkey::Pubkey;
    /// # let now: i64 = 0;
    /// let currency_mint = Pubkey::new_unique();
    /// let mut project = ProjectBuilder::new()
    ///                    .id("Euro BANGK", "EUB", "https://bangk.app/eurobangk")
    ///                    .unwrap()
    ///                    .seep_bump(255)
    ///                    .ata(&Pubkey::new_unique())
    ///                    .interest_rate((7.5_f64 * 1e6_f64).trunc() as u32)
    ///                    .unwrap()
    ///                    .token_value(1)
    ///                    .unwrap()
    ///                    .payment_periodicity(Periodicity::Monthly)
    ///                    .risk_assessment(1)
    ///                    .unwrap()
    ///                    .build();
    /// project.launch(now);
    /// ```
    pub fn launch(&mut self, timestamp: i64) -> Result<&mut Self, Error> {
        self.status = ProjectStatus::Live;
        self.next_payment = self
            .get_next_payment_time(timestamp)
            .ok_or(Error::InvalidProjectArgument)?;
        Ok(self)
    }

    /// Changes the status of a project from Open to Canceled.
    ///
    /// Example
    /// ```
    /// # use bangk::state::projects::Project;
    /// # use bangk::state::projects::ProjectBuilder;
    /// # use bangk::state::projects::Periodicity;
    /// # use solana_program::pubkey::Pubkey;
    /// let currency_mint = Pubkey::new_unique();
    /// let mut project = ProjectBuilder::new()
    ///                       .id("SonikCoin", "SKC", "https://sonikcoin")
    ///                       .unwrap()
    ///                       .seep_bump(254)
    ///                       .ata(&currency_mint)
    ///                       .interest_rate((2.1_f64 * 1e6_f64).trunc() as u32)
    ///                       .unwrap()
    ///                       .token_value(1)
    ///                       .unwrap()
    ///                       .payment_periodicity(Periodicity::Quarterly)
    ///                       .risk_assessment(7)
    ///                       .unwrap()
    ///                       .build();
    /// project.cancel();
    /// ```
    pub fn cancel(&mut self) -> &mut Self {
        self.status = ProjectStatus::Cancelled;
        self
    }

    /// Changes the status of a project from Live to Closed.
    ///
    /// Example
    /// ```
    /// # use bangk::state::projects::Project;
    /// # use bangk::state::projects::ProjectBuilder;
    /// # use bangk::state::projects::Periodicity;
    /// # use solana_program::pubkey::Pubkey;
    /// # let now: i64 = 0;
    /// let currency_mint = Pubkey::new_unique();
    /// let mut project = ProjectBuilder::new()
    ///                       .id("Euro BANGK", "EUB", "https://bangk.app/eurobangk")
    ///                       .unwrap()
    ///                       .seep_bump(255)
    ///                       .ata(&Pubkey::new_unique())
    ///                       .interest_rate((7.5_f64 * 1e6_f64).trunc() as u32)
    ///                       .unwrap()
    ///                       .token_value(1)
    ///                       .unwrap()
    ///                       .payment_periodicity(Periodicity::Weekly)
    ///                       .risk_assessment(1)
    ///                       .unwrap()
    ///                       .build();
    /// project.launch(now);
    /// project.close();
    /// ```
    pub fn close(&mut self) -> &mut Self {
        self.status = ProjectStatus::Closed;
        self.next_payment = 0;
        self
    }

    ///
    /// As soon as a round of interest payment is done for the period,
    /// the new date of the next payment is computed from the current next one,
    /// and **not** from the current date (to not shift anything if there was a
    /// delay at some point).
    /// The current date however becomes the date of the last payment.
    ///
    /// # Errors
    /// If the next payment date could not be properly computed.
    ///
    /// Example
    /// ```
    /// # use bangk::state::projects::Project;
    /// # use bangk::state::projects::ProjectBuilder;
    /// # use bangk::state::projects::Periodicity;
    /// # use solana_program::pubkey::Pubkey;
    /// # let now: i64 = 0;
    /// let currency_mint = Pubkey::new_unique();
    /// let mut project = ProjectBuilder::new()
    ///                       .id("Euro BANGK", "EUB", "https://bangk.app/eurobangk")
    ///                       .unwrap()
    ///                       .seep_bump(255)
    ///                       .ata(&Pubkey::new_unique())
    ///                       .interest_rate((7.5_f64 * 1e6_f64).trunc() as u32)
    ///                       .unwrap()
    ///                       .token_value(1)
    ///                       .unwrap()
    ///                       .payment_periodicity(Periodicity::Daily)
    ///                       .risk_assessment(1)
    ///                       .unwrap()
    ///                       .build();
    /// project.launch(now);
    /// // Pay the first interests here
    /// project.update_payment_dates(now);
    /// ```
    pub fn update_payment_dates(&mut self, timestamp: i64) -> Result<&mut Self, Error> {
        self.last_payment = self.next_payment;
        self.next_payment = self
            .get_next_payment_time(timestamp)
            .ok_or(Error::InvalidProjectArgument)?;
        Ok(self)
    }

    /// Computes the next payment date from the last payment and the payment periodicity.
    /// If no payment has been made yet, the current date is used as a basis.
    fn get_next_payment_time(&self, timestamp: i64) -> Option<i64> {
        let date: DateTime<Utc> = DateTime::from_timestamp(
            if self.last_payment > 0 {
                self.last_payment
            } else {
                timestamp
            },
            0,
        )?;
        let date = match self.payment_periodicity {
            Periodicity::Daily => date.checked_add_days(Days::new(1))?,
            Periodicity::Weekly => date.checked_add_days(Days::new(7))?,
            Periodicity::Monthly => date.checked_add_months(Months::new(1))?,
            Periodicity::Quarterly => date.checked_add_months(Months::new(3))?,
            Periodicity::BiAnnually => date.checked_add_months(Months::new(6))?,
            Periodicity::Annually => date.checked_add_months(Months::new(12))?,
        };

        Some(date.timestamp())
    }
}

/// Builder for a new project.
#[derive(Default)]
pub struct ProjectBuilder {
    project: Project,
}

impl ProjectBuilder {
    /// Initializes a new project builder object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            project: Project::default(),
        }
    }

    /// Sets the project's ID triplets: id, symbol and `URI`
    ///
    /// # Parameters
    /// * `id` - Unique name of the project, used to derive the seeds,
    /// * `symbol` - Symbol of the project's token,
    /// * `uri` - `URI` (of an image for example) of the token,
    ///
    /// # Errors
    /// If the strings are too long (name and `URI` >= 128 or Symbol >= 5).
    pub fn id<T, S, R>(mut self, id: T, symbol: S, uri: R) -> Result<Self, Error>
    where
        T: Into<String>,
        S: Into<String>,
        R: Into<String>,
    {
        let id = id.into();
        let symbol = symbol.into();
        let uri = uri.into();
        if id.len() > 128 || symbol.len() > 5 || uri.len() > 128 {
            return Err(Error::ArgumentTooLong);
        }
        self.project.name = id;
        self.project.symbol = symbol;
        self.project.uri = uri;
        Ok(self)
    }

    /// Sets the project's PDA seed bump.
    #[must_use]
    pub const fn seep_bump(mut self, bump: u8) -> Self {
        self.project.seed_bump = bump;
        self
    }

    /// Sets the project's associated ATA.
    ///
    /// This ATA is the only one allowed to make or receive payment for the project.
    #[must_use]
    pub fn ata(mut self, ata: &Pubkey) -> Self {
        self.project.ata = ata.to_string();
        self
    }

    /// Sets the project's target interest rate.
    ///
    /// # Errors
    /// If the interest rate is 0.
    ///
    /// # Errors
    /// If the interest rate is 0.
    pub fn interest_rate(mut self, interest_rate: u32) -> Result<Self, Error> {
        if interest_rate == 0 {
            return Err(Error::InvalidProjectArgument);
        }
        self.project.interest_rate = interest_rate;
        Ok(self)
    }

    /// Sets the project's token value (in stable coins).
    ///
    /// # Errors
    /// If the token value is 0.
    ///
    /// # Errors
    /// If the token value is 0.
    pub fn token_value(mut self, token_value: u32) -> Result<Self, Error> {
        if token_value == 0 {
            return Err(Error::InvalidProjectArgument);
        }
        self.project.token_value = token_value;
        Ok(self)
    }

    /// Sets the project's interest rate payment periodicity.
    #[must_use]
    pub const fn payment_periodicity(mut self, periodicity: Periodicity) -> Self {
        self.project.payment_periodicity = periodicity;
        self
    }

    /// Sets the project's risk assessment.
    ///
    /// # Errors
    /// If the risks are not between 1 and 7 (included).
    ///
    /// # Errors
    /// If the risk assessment is not in \[1,7\].
    pub fn risk_assessment(mut self, risk: u8) -> Result<Self, Error> {
        if risk == 0 || risk > 7 {
            return Err(Error::InvalidProjectArgument);
        }
        self.project.risk_assessment = risk;
        Ok(self)
    }

    /// Finalizes the project's construction.
    ///
    /// The mint is *not* yet written on the blockchain.
    #[must_use]
    pub fn build(self) -> Project {
        self.project
    }
}

impl TryFrom<TokenMetadata> for Project {
    type Error = Error;

    fn try_from(value: TokenMetadata) -> Result<Self, Self::Error> {
        let mut res = Self {
            name: value.name,
            symbol: value.symbol,
            uri: value.uri,
            ..Default::default()
        };

        for (key, data) in value.additional_metadata {
            match key.as_str() {
                "rate" => {
                    res.interest_rate =
                        data.parse().map_err(|_err| Error::InvalidProjectArgument)?;
                }
                "last" => {
                    res.last_payment =
                        data.parse().map_err(|_err| Error::InvalidProjectArgument)?;
                }
                "next" => {
                    res.next_payment =
                        data.parse().map_err(|_err| Error::InvalidProjectArgument)?;
                }
                "value" => {
                    res.token_value = data.parse().map_err(|_err| Error::InvalidProjectArgument)?;
                }
                "risk" => {
                    res.risk_assessment =
                        data.parse().map_err(|_err| Error::InvalidProjectArgument)?;
                }
                "period" => {
                    res.payment_periodicity = data.parse::<u8>().unwrap_or_default().try_into()?;
                }
                "status" => res.status = data.parse::<u8>().unwrap_or_default().try_into()?,
                "ata" => {
                    res.ata = data;
                }
                _ => return Err(Error::InvalidProjectArgument),
            }
        }
        Ok(res)
    }
}

impl TryFrom<u8> for Periodicity {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Error> {
        Ok(match value {
            0 => Self::Daily,
            1 => Self::Weekly,
            2 => Self::Monthly,
            3 => Self::Quarterly,
            4 => Self::BiAnnually,
            5 => Self::Annually,
            _ => return Err(Error::InvalidProjectArgument),
        })
    }
}

impl TryFrom<u8> for ProjectStatus {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Error> {
        Ok(match value {
            0 => Self::Open,
            1 => Self::Live,
            2 => Self::Closed,
            3 => Self::Cancelled,
            _ => return Err(Error::InvalidProjectArgument),
        })
    }
}

impl AsRef<Self> for Project {
    fn as_ref(&self) -> &Self {
        self
    }
}

/// Check that the given ATA is the expected one for the project.
///
/// # Parameters
/// * `ata` - ATA to check
///
/// # Errors
/// If the ATA is *not* the expected one.
pub fn check_project_ata(project: &Project, ata: &AccountInfo) -> Result<(), ProgramError> {
    debug!(
        "checking if {} is same as expected ({})",
        ata.key, project.ata
    );
    if ata.key.to_string() == project.ata {
        Ok(())
    } else {
        Err(Error::InvalidAta.into())
    }
}

impl<'a> MintData<'a> for Project {
    fn get_symbol(&self) -> String {
        self.symbol.clone()
    }

    fn base_seed(symbol: &str) -> String {
        format!("Project{symbol}")
    }

    fn to_metadata(&self, account: &AccountInfo<'a>) -> Result<TokenMetadata, ProgramError> {
        let key_values = vec![
            (String::from("ata"), self.ata.clone()),
            (String::from("rate"), format!("{:05}", self.interest_rate)),
            (String::from("last"), format!("{:010}", self.last_payment)),
            (String::from("next"), format!("{:010}", self.next_payment)),
            (String::from("value"), format!("{:010}", self.token_value)),
            (String::from("risk"), self.risk_assessment.to_string()),
            (
                String::from("period"),
                (self.payment_periodicity as u8).to_string(),
            ),
            (String::from("status"), (self.status as u8).to_string()),
        ];
        token_metadata(account.key, &self.name, &self.symbol, &self.uri, key_values)
    }
}

//////////////////////////////////////
//              Tests               //
//////////////////////////////////////

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::float_arithmetic)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn create_project() -> Project {
        ProjectBuilder::new()
            .id("TEST-001", "SYM", "https://no.where")
            .unwrap()
            .seep_bump(255)
            .ata(&Pubkey::new_unique())
            .interest_rate((7.5_f64 * 1e6_f64).trunc() as u32)
            .unwrap()
            .token_value(1)
            .unwrap()
            .payment_periodicity(Periodicity::Monthly)
            .risk_assessment(1)
            .unwrap()
            .build()
    }

    #[test]
    fn seeds() {
        let project = create_project();
        let (mint, bump) = Project::get_address(&project.symbol);
        assert_eq!(
            mint.to_string(),
            "AgGsYnYrt8zfWKQHEtTckd3tEJsX11giykQ7M1F3796P",
            "project mint address is wrong"
        );
        assert_eq!(bump, 255, "seed bump is wrong");
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[test]
    fn creation() {
        let project = create_project();
        assert_eq!(project.name, "TEST-001", "ID is wrong");
        assert_eq!(project.symbol, "SYM", "Symbol is wrong");
        assert_eq!(project.uri, "https://no.where", "URI is wrong");
        assert_eq!(project.seed_bump, 255, "seed bump is wrong");
        assert_eq!(
            project.interest_rate,
            (7.5_f64 * 1e6_f64).trunc() as u32,
            "interest rate is wrong"
        );
        assert_eq!(project.token_value, 1, "token value is wrong");
        assert_eq!(
            project.payment_periodicity,
            Periodicity::Monthly,
            "payment periodicity is wrong"
        );
        assert_eq!(project.risk_assessment, 1, "risk assessment is wrong");
        assert_eq!(project.last_payment, 0, "last payment is wrong");
        assert_eq!(project.next_payment, 0, "next payment is wrong");
        assert_eq!(project.status, ProjectStatus::Open, "status is wrong");
    }

    #[test]
    fn creation_invalid_args() {
        assert!(ProjectBuilder::new()
            .id("TEST-001", "this is not valid", "https://no.where")
            .is_err());
        assert!(ProjectBuilder::new().interest_rate(0).is_err());
        assert!(ProjectBuilder::new().token_value(0).is_err());
        assert!(ProjectBuilder::new().risk_assessment(0).is_err());
        assert!(ProjectBuilder::new().risk_assessment(10).is_err());
    }

    #[test]
    fn launch() {
        let mut project = create_project();
        let now = Utc::now().timestamp();
        let success = project.launch(now).is_ok();
        assert!(success, "could not launch the project");
        assert!(
            project.next_payment >= now,
            "next payment was not computed correctly"
        );
        assert_eq!(
            project.status,
            ProjectStatus::Live,
            "status was not updated"
        );
    }

    #[test]
    fn cancel() {
        let mut project = create_project();
        project.cancel();
        assert_eq!(
            project.status,
            ProjectStatus::Cancelled,
            "status was not updated correctly"
        );
    }

    #[test]
    fn close() {
        let mut project = create_project();
        let now = Utc::now().timestamp();
        let success = project.launch(now).is_ok();
        assert!(success, "could not launch the project");
        assert!(project.next_payment != 0);
        project.close();
        assert_eq!(
            project.next_payment, 0,
            "next payment was not updated correctly"
        );
        assert_eq!(
            project.status,
            ProjectStatus::Closed,
            "status was not updated correctly"
        );
    }

    #[test]
    fn next_payment() {
        let now = Utc::now().timestamp();
        let mut project = create_project();
        project.last_payment = Utc
            .with_ymd_and_hms(2024, 2, 11, 12, 0, 0)
            .unwrap()
            .timestamp();
        project.payment_periodicity = Periodicity::Daily;
        assert_eq!(
            project.get_next_payment_time(now),
            Some(
                Utc.with_ymd_and_hms(2024, 2, 12, 12, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
        project.payment_periodicity = Periodicity::Weekly;
        assert_eq!(
            project.get_next_payment_time(now),
            Some(
                Utc.with_ymd_and_hms(2024, 2, 18, 12, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
        project.payment_periodicity = Periodicity::Monthly;
        assert_eq!(
            project.get_next_payment_time(now),
            Some(
                Utc.with_ymd_and_hms(2024, 3, 11, 12, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
        project.payment_periodicity = Periodicity::Quarterly;
        assert_eq!(
            project.get_next_payment_time(now),
            Some(
                Utc.with_ymd_and_hms(2024, 5, 11, 12, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
        project.payment_periodicity = Periodicity::BiAnnually;
        assert_eq!(
            project.get_next_payment_time(now),
            Some(
                Utc.with_ymd_and_hms(2024, 8, 11, 12, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
        project.payment_periodicity = Periodicity::Annually;
        assert_eq!(
            project.get_next_payment_time(now),
            Some(
                Utc.with_ymd_and_hms(2025, 2, 11, 12, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
    }

    #[test]
    fn update_payment_dates() {
        let now = Utc::now().timestamp();
        let mut project = create_project();
        let launch_success = project.launch(now).is_ok();
        assert!(launch_success, "could not launch the project");

        let old_next = project.next_payment;
        let date_success = project.update_payment_dates(now).is_ok();
        assert!(date_success, "could not get next dates");

        assert_eq!(
            project.last_payment, old_next,
            "payment date was not set successfully"
        );

        assert_eq!(
            project.next_payment,
            DateTime::<Utc>::from_timestamp(old_next, 0)
                .unwrap_or_default()
                .checked_add_months(Months::new(1))
                .unwrap_or_default()
                .timestamp(),
            "next payment was not updated correctly (old_next is {old_next})"
        );
    }
}
