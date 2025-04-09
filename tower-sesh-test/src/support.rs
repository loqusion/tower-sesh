use std::time::Duration;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, Month, OffsetDateTime, Time, UtcDateTime};
use tower_sesh_core::Ttl;

pub use rand_chacha::ChaCha20Rng as TestRng;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct SessionData {
    pub user_id: DbId,
    pub authenticated: bool,
    pub roles: Vec<String>,
    pub preferences: Preferences,
    pub cart: Vec<CartItem>,
    pub csrf_token: String,
    pub flash_messages: Vec<String>,
    pub rate_limit: RateLimit,
    pub workflow_state: WorkflowState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DbId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Preferences {
    pub theme: Theme,
    pub language: Language,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Theme {
    Light,
    Dark,
}

/// The two languages
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    #[serde(alias = "en-US")]
    EnUs,
    #[serde(alias = "en-GB")]
    EnGb,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct CartItem {
    pub item_id: DbId,
    pub name: String,
    pub quantity: u64,
    pub price: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct RateLimit {
    pub failed_login_attempts: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub last_attempt: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WorkflowState {
    pub step: u64,
    pub total_steps: u64,
    pub data: WorkflowData,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WorkflowData {
    pub address: String,
}

impl SessionData {
    pub fn sample() -> Self {
        SessionData::sample_with(12345)
    }

    pub fn sample_with(user_id: u64) -> Self {
        SessionData {
            user_id: DbId(user_id),
            authenticated: true,
            roles: vec!["admin".to_owned(), "editor".to_owned()],
            preferences: Preferences {
                theme: Theme::Dark,
                language: Language::EnUs,
            },
            cart: vec![
                CartItem {
                    item_id: DbId(101),
                    name: "Laptop".to_owned(),
                    quantity: 1,
                    price: Decimal::new(99999, 2),
                },
                CartItem {
                    item_id: DbId(202),
                    name: "Mouse".to_owned(),
                    quantity: 2,
                    price: Decimal::new(2550, 2),
                },
            ],
            csrf_token: "abc123xyz".to_owned(),
            flash_messages: vec![
                "Welcome back!".to_owned(),
                "Your order has been placed successfully.".to_owned(),
            ],
            rate_limit: RateLimit {
                failed_login_attempts: 1,
                last_attempt: OffsetDateTime::new_utc(
                    Date::from_calendar_date(2025, Month::February, 28).unwrap(),
                    Time::from_hms(0, 34, 56).unwrap(),
                ),
            },
            workflow_state: WorkflowState {
                step: 2,
                total_steps: 5,
                data: WorkflowData {
                    address: "123 Main St, NY".to_owned(),
                },
            },
        }
    }
}

/// Returns a `Ttl` that will not expire.
///
/// (Technically, the returned `Ttl` will expire if a test runs for longer than
/// 10 minutes.)
pub(crate) fn ttl() -> Ttl {
    let now = Ttl::now_local().unwrap();
    now + Duration::from_secs(10 * 60)
}

/// Returns a `Ttl` that is very close to expiring.
pub(crate) fn ttl_strict() -> Ttl {
    let now = Ttl::now_local().unwrap();
    ttl_strict_of(now)
}

/// Returns a `Ttl` that is very close to expiring, relative to `ttl`.
pub(crate) fn ttl_strict_of(ttl: Ttl) -> Ttl {
    // miri requires a more lenient TTL due to its slower execution speed
    const STRICT_OFFSET: Duration = if cfg!(miri) {
        Duration::from_secs(20)
    } else {
        // NOTE: This threshold may cause spurious test failures on some
        // systems. If that is the case, try increasing this value.
        Duration::from_millis(1500)
    };
    ttl + STRICT_OFFSET
}

/// Returns a `Ttl` that has already expired.
pub(crate) fn ttl_expired() -> Ttl {
    let now = Ttl::now_local().unwrap();
    now - Duration::from_secs(1)
}

/// A trait implementing test utility functions for [`Ttl`].
pub(crate) trait TtlExt {
    type Normalized;

    fn normalize(self) -> Self::Normalized;
}

impl TtlExt for Ttl {
    type Normalized = UtcDateTime;

    /// Returns a normalized form of a [`Ttl`].
    ///
    /// This truncates the nanosecond portion of the `Ttl` and converts to UTC.
    fn normalize(self) -> Self::Normalized {
        self.replace_nanosecond(0).unwrap().to_utc()
    }
}
