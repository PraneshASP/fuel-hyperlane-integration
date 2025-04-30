mod asset_send_with_gas;
mod collateral_asset_recieve;
mod collateral_asset_send;
mod gas_overpayment_and_claim;
mod hooks_setup;
mod message_recieve;
mod message_send_with_gas;
mod native_asset_recieve;
mod native_asset_send;
mod remote_mailbox;
mod set_gas_configs;
mod synthetic_asset_recieve;
mod synthetic_asset_send;
mod wrapped_asset_mint_test;
mod wrapped_assets_full_flow;
//mod wrapped_assets_send;

use std::{future::Future, pin::Pin};

type TestFn = Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<f64, String>>>>>;

pub struct TestCase {
    name: String,
    test: TestFn,
}

impl TestCase {
    pub fn new<F, Fut>(name: &str, test: F) -> Self
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<f64, String>> + 'static,
    {
        Self {
            name: name.to_string(),
            test: Box::new(move || Box::pin(test())),
        }
    }

    pub async fn run(self) -> Result<f64, String> {
        (self.test)().await
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

pub struct FailedTestCase {
    name: String,
    error: String,
}

impl FailedTestCase {
    pub fn new(name: String, error: String) -> Self {
        Self { name, error }
    }

    pub fn log(&self) {
        println!("Test {} failed: {}", self.name, self.error);
    }
}

pub fn pull_test_cases() -> Vec<TestCase> {
    vec![
        set_gas_configs::test(),
        message_send_with_gas::test(),
        remote_mailbox::test(),
        collateral_asset_send::test(),
        native_asset_send::test(),
        synthetic_asset_send::test(),
        gas_overpayment_and_claim::test(),
        asset_send_with_gas::test(),
        message_recieve::test(),
        synthetic_asset_recieve::test(),
        collateral_asset_recieve::test(),
        native_asset_recieve::test(),
        hooks_setup::test(),
        // wrapped_asset_mint_test::test(),
       wrapped_assets_full_flow::test(),
       // wrapped_assets_send::test(),
    ]
}
