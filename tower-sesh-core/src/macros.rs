#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::error!($($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::warn!($($arg)*);
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::info!($($arg)*);
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::debug!($($arg)*);
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        $crate::__private::tracing::trace!($($arg)*);
    };
}
