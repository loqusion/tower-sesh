#![allow(unused_macros)]

macro_rules! __log_rejection {
    (
        rejection_type = $ty:ident,
        body_text = $body_text:expr,
        status = $status:expr,
    ) => {
        {
            #[cfg(feature = "tracing")]
            ::tracing::event!(
                target: "tower_sesh::rejection",
                ::tracing::Level::TRACE,
                status = $status.as_u16(),
                body = $body_text,
                rejection_type = ::core::any::type_name::<$ty>(),
                "rejecting request",
            );
        }
    };
}
pub(crate) use __log_rejection;

macro_rules! define_rejection {
    (
        #[status = $status:ident]
        #[body = $body:expr]
        $(#[$m:meta])*
        pub struct $name:ident;
    ) => {
        #[cfg(feature = "axum")]
        $(#[$m])*
        #[derive(::core::fmt::Debug)]
        #[non_exhaustive]
        pub struct $name;

        #[cfg(feature = "axum")]
        impl ::axum::response::IntoResponse for $name {
            fn into_response(self) -> ::axum::response::Response {
                $crate::macros::axum::__log_rejection!(
                    rejection_type = $name,
                    body_text = $body,
                    status = ::http::StatusCode::$status,
                );
                ::axum::response::IntoResponse::into_response((self.status(), $body))
            }
        }

        #[cfg(feature = "axum")]
        impl $name {
            /// Get the response body text used for this rejection.
            #[allow(dead_code)]
            pub fn body_text(&self) -> ::std::string::String {
                ::std::string::String::from($body)
            }

            /// Get the status code used for this rejection.
            pub fn status(&self) -> ::http::StatusCode {
                ::http::StatusCode::$status
            }
        }

        #[cfg(feature = "axum")]
        impl ::core::fmt::Display for $name {
            fn fmt(&self, __formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::write!(__formatter, "{}", $body)
            }
        }

        #[cfg(feature = "axum")]
        impl ::std::error::Error for $name {}

        #[cfg(feature = "axum")]
        impl ::core::default::Default for $name {
            fn default() -> Self {
                Self
            }
        }
    };

    (
        #[status = $status:ident]
        #[body = $body:expr]
        $(#[$m:meta])*
        pub struct $name:ident (Error);
    ) => {
        #[cfg(feature = "axum")]
        $(#[$m])*
        #[derive(::core::fmt::Debug)]
        pub struct $name(pub(crate) ::axum::Error);

        #[cfg(feature = "axum")]
        impl $name {
            #[allow(dead_code)]
            pub(crate) fn from_err<E>(err: E) -> Self
            where
                E: ::core::convert::Into<::axum::BoxError>,
            {
                Self(::axum::Error::new(err))
            }
        }

        #[cfg(feature = "axum")]
        impl ::axum::response::IntoResponse for $name {
            fn into_response(self) -> ::axum::response::Response {
                $crate::macros::axum::__log_rejection!(
                    rejection_type = $name,
                    body_text = self.body_text(),
                    status = ::http::StatusCode::$status,
                );
                ::axum::response::IntoResponse::into_response((
                    self.status(),
                    self.body_text()
                ))
            }
        }

        #[cfg(feature = "axum")]
        impl $name {
            /// Get the response body text used for this rejection.
            pub fn body_text(&self) -> ::std::string::String {
                ::std::format!(::core::concat!($body, ": {}"), self.0)
            }

            /// Get the status code used for this rejection.
            pub fn status(&self) -> ::http::StatusCode {
                ::http::StatusCode::$status
            }
        }

        #[cfg(feature = "axum")]
        impl ::core::fmt::Display for $name {
            fn fmt(&self, __formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::write!(__formatter, "{}", $body)
            }
        }

        #[cfg(feature = "axum")]
        impl ::std::error::Error for $name {
            fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                ::core::option::Option::Some(&self.0)
            }
        }
    };
}
