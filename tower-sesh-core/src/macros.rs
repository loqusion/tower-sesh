/// Logs a message at the error level.
///
/// This macro delegates to [`tracing::error`].
///
/// If the `tracing` feature is not enabled, this is a no-op.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::error!($($arg)*);
    };
}

/// Logs a message at the warn level.
///
/// This macro delegates to [`tracing::warn`].
///
/// If the `tracing` feature is not enabled, this is a no-op.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::warn!($($arg)*);
    };
}

/// Logs a message at the info level.
///
/// This macro delegates to [`tracing::info`].
///
/// If the `tracing` feature is not enabled, this is a no-op.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::info!($($arg)*);
    };
}

/// Logs a message at the debug level.
///
/// This macro delegates to [`tracing::debug`].
///
/// If the `tracing` feature is not enabled, this is a no-op.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::debug!($($arg)*);
    };
}

/// Logs a message at the trace level.
///
/// This macro delegates to [`tracing::trace`].
///
/// If the `tracing` feature is not enabled, this is a no-op.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::trace!($($arg)*);
    };
}
