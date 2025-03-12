#[macro_export]
macro_rules! test_suite {
    ($store:expr) => {
        #[test]
        fn smoke() {
            $crate::smoke();
        }
    };
}

pub fn smoke() {}
