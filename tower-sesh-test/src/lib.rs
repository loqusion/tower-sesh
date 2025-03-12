use tower_sesh_core::SessionStore;

#[doc(hidden)]
pub use paste;

#[macro_export]
macro_rules! test_suite {
    ($store:expr) => {
        $crate::test_suite! {
            @impl $store =>
            smoke
        }
    };

    (@impl $store:expr => $($test:ident)+) => {
        $(
            #[tokio::test]
            async fn $test() {
                $crate::paste::paste!{
                    $crate::[<test_ $test>]($store).await;
                }
            }
        )+
    };
}

pub async fn test_smoke(_store: impl SessionStore<()>) {}
