use time::{OffsetDateTime, UtcOffset};

/// An instant in time, represented as a date and time with a timezone offset.
///
/// Used to represent a session's expiration time, after which a
/// [`SessionStore`] implementation should delete the session.
///
/// [`SessionStore`]: crate::SessionStore
pub type Ttl = OffsetDateTime;

const WEEK_IN_SECONDS: u32 = 60 * 60 * 24 * 7;
/// Default expiry offset for a session, in seconds.
pub const SESSION_EXPIRY_SECONDS_DEFAULT: u32 = 2 * WEEK_IN_SECONDS;

/// Returns the current date and time with the local system's UTC offset.
///
/// If the system's UTC offset could not be found, then [`now_utc`] is used
/// instead.
///
/// [`now_utc`]: Ttl::now_utc
#[inline]
pub fn now() -> Ttl {
    let t = Ttl::now_utc();
    match UtcOffset::local_offset_at(t) {
        Ok(offset) => t.to_offset(offset),
        Err(_err) => t,
    }
}
