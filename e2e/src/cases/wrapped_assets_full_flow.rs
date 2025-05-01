use crate::{
    cases::TestCase,
    evm::{get_evm_wallet, monitor_fuel_for_delivery, SepoliaContracts},
    setup::{
        abis::{AssetRegistry, Mailbox, PostDispatchHook, WrappedAssetMinter},
        get_loaded_wallet,
    },
    utils::{
        get_evm_domain, get_fuel_domain,
        local_contracts::{get_contract_address_from_yaml, load_remote_wr_addresses},
        token::{get_balance, get_contract_balance, },
    },
};
use alloy::{
    network::NetworkWallet,
    primitives::{FixedBytes, U256},
};
use fuels::{
    programs::calls::{CallParameters, Execution},
    types::{transaction_builders::VariableOutputPolicy, Bits256, Bytes, ContractId, Identity},
};
use tokio::time::Instant;

pub fn test() -> TestCase {
    TestCase::new(
        "wrapped_assets_full_flow",
        wrapped_assets_full_flow,
    )
}

async fn wrapped_assets_full_flow() -> std::result::Result<f64, String> {
    let start = Instant::now();

    let wallet = get_loaded_wallet().await;

    let mailbox_id = get_contract_address_from_yaml("mailbox");
    let hook_id = get_contract_address_from_yaml("postDispatch");
    let minter_id = get_contract_address_from_yaml("wrappedAssetMinter");
    let registry_id = get_contract_address_from_yaml("assetRegistry");
    let ism_id = get_contract_address_from_yaml("interchainSecurityModule");
    let igp_id = get_contract_address_from_yaml("interchainGasPaymaster");
    let gas_oracle_id = get_contract_address_from_yaml("gasOracle");

    let minter = WrappedAssetMinter::new(minter_id.clone(), wallet.clone());
    let registry = AssetRegistry::new(registry_id.clone(), wallet.clone());
    let mailbox = Mailbox::new(mailbox_id.clone(), wallet.clone());
    let hook = PostDispatchHook::new(hook_id.clone(), wallet.clone());

    let _ = minter
        .methods()
        .set_registry(Bits256(registry_id.into()))
        .call()
        .await;
    let minter_reg = registry
        .methods()
        .get_minter()
        .simulate(Execution::StateReadOnly)
        .await
        .unwrap()
        .value;
    println!("Minter address: {}", minter_reg);
    println!("Setting up registry and contracts...");

    let bridge_id = registry
        .methods()
        .register_bridge(
            "HyperlaneBridge".to_string(),
            Identity::ContractId(mailbox_id.into()),
        )
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;

    let is_bridge_registered = registry
        .methods()
        .is_bridge_registered(bridge_id)
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;

    assert!(is_bridge_registered, "Bridge was not registered");
    println!("Bridge registered successfully with ID: {:?}", bridge_id);

    let origin_chain_id = get_evm_domain();
    // Should use CTR instead??
    let token_address_hex = "0x0000000000000000000000009E545E3C0baAB3E08CdfD552C960A1050f373042";

    let token_address = Bits256::from_hex_str(token_address_hex);

    println!("Registering token on Fuel...");

    let sub_id = registry
        .methods()
        .register_asset(
            origin_chain_id as u64,
            token_address.unwrap(),
            9, // Token decimals
            "Universal Wrapped Token".into(),
            "UWT".into(),
        )
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;

    registry
        .methods()
        .authorize_bridge_for_asset(bridge_id, sub_id)
        .call()
        .await
        .map_err(|e| e.to_string())?;

    let redemption_ticket_id = registry
        .methods()
        .get_redemption_ticket_for_sub_id(sub_id)
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;

    assert!(redemption_ticket_id.is_some());
    println!("Asset registered with sub_id: {:?}", sub_id);
    println!(
        "Redemption ticket ID: {:?}",
        redemption_ticket_id.unwrap()
    );

    let remote_wallet = get_evm_wallet().await;
    let contracts = SepoliaContracts::initialize(remote_wallet.clone()).await;
    let remote_wr = contracts.warp_route_collateral; // Using the collateral route
    let remote_mailbox = contracts.mailbox;
    println!(
        "Wallet address : {:?}",
        remote_wallet
            .default_signer()
            .address()
            .to_checksum(Some(1u64))
    );
    println!("remote_wr : {:?}", remote_wr.address());

    let fuel_domain = get_fuel_domain();
    let fuel_registry_bytes = FixedBytes::from_slice(registry_id.as_slice());

    println!("Setting up bidirectional router enrollment...");

    let remote_wr_address = load_remote_wr_addresses("CTR").unwrap(); // Collateral token router
    let remote_wr_hex = hex::decode(remote_wr_address.strip_prefix("0x").unwrap()).unwrap();
    let mut remote_wr_array = [0u8; 32];
    remote_wr_array[12..].copy_from_slice(&remote_wr_hex);
    println!("Bits256(remote_wr_array): {:?}", Bits256(remote_wr_array));
    registry
        .methods()
        .enroll_remote_router(origin_chain_id, Bits256(remote_wr_array))
        .call()
        .await
        .map_err(|e| format!("Failed to enroll remote router on Fuel: {:?}", e))?;

    registry
        .methods()
        .set_remote_router_decimals(Bits256(remote_wr_array), 18) // Sepolia uses 18 decimals
        .call()
        .await
        .map_err(|e| format!("Failed to set remote router decimals on Fuel: {:?}", e))?;

    let _ = remote_wr
        .enrollRemoteRouter(fuel_domain, fuel_registry_bytes)
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .map_err(|e| format!("Failed to enroll remote router on Sepolia: {:?}", e))?;

    println!("Bidirectional router enrollment complete");

    let amount = 1_000_000u64;

    let recipient = FixedBytes::from_slice(wallet.address().hash().as_slice());

    let quote_dispatch = remote_wr
        .quoteGasPayment(fuel_domain)
        .call()
        .await
        .unwrap()
        ._0;

    println!("Got gas quote: {:?}", quote_dispatch);

    let _tr = remote_wr
        .transferRemote_1(fuel_domain, recipient, U256::from(amount))
        .value(quote_dispatch + U256::from(amount)) // Pay for gas + token amount
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .map_err(|e| format!("Failed to transfer remote: {:?}", e))?;

    println!("Sent transfer from Sepolia to Fuel");

    let msg_id = remote_mailbox.latestDispatchedId().call().await.unwrap()._0;

    if FixedBytes::const_is_zero(&msg_id) {
        return Err("Failed to get valid message ID".to_string());
    }

    println!("Got message ID: {:?}", msg_id);

    let delivery_success = monitor_fuel_for_delivery(mailbox.clone(), msg_id).await;
    assert!(delivery_success, "Message was not delivered to Fuel");
    println!("Message was delivered to Fuel");

    let wrapped_asset_id = minter.contract_id().asset_id(&sub_id);
    let redemption_ticket_asset_id = minter
        .contract_id()
        .asset_id(&redemption_ticket_id.unwrap());

    let wrapped_balance = get_balance(
        wallet.provider().unwrap(),
        wallet.address(),
        wrapped_asset_id,
    )
    .await
    .unwrap();

    let redemption_balance = get_balance(
        wallet.provider().unwrap(),
        wallet.address(),
        redemption_ticket_asset_id,
    )
    .await
    .unwrap();

    assert_eq!(
        wrapped_balance, amount,
        "Wrapped token balance is incorrect"
    );
    assert_eq!(
        redemption_balance, amount,
        "Redemption ticket balance is incorrect"
    );

    let total_supply = minter
        .methods()
        .total_supply(wrapped_asset_id)
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;

    assert_eq!(
        total_supply.unwrap_or(0),
        amount
    );

    println!("Wrapped Asset balance: {}", wrapped_balance);
    println!("Redemption ticket balance: {}", redemption_balance);

    println!("✅ wrapped_assets_bidirectional_flow test passed");
    Ok(start.elapsed().as_secs_f64())
}
