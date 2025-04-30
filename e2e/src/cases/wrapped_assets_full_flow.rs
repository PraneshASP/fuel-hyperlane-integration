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
        token::{get_balance, get_contract_balance, send_gas_to_contract_2},
    },
};
use alloy::primitives::{FixedBytes, U256};
use fuels::{
    types::{transaction_builders::VariableOutputPolicy, Bits256, ContractId, Identity, Bytes}, 
    programs::calls::CallParameters,
};
use tokio::time::Instant;

pub fn test() -> TestCase {
    TestCase::new("wrapped_assets_bidirectional_flow", wrapped_assets_bidirectional_flow)
}

async fn wrapped_assets_bidirectional_flow() -> std::result::Result<f64, String> {
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

    let _ = minter.methods().set_registry(Bits256(registry_id.into())).call().await;
    
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
    
    assert!(is_bridge_registered, "Bridge was not registered correctly");


    let origin_chain_id = get_evm_domain();  
    
    let token_address_hex = "a513E6E4b8f2a923D98304ec87F64353C4D5C853"; // Collateral token on Sepolia
    let raw = hex::decode(token_address_hex).map_err(|e| e.to_string())?;
    
    let mut token_bytes = [0u8; 32];
    token_bytes[12..].copy_from_slice(&raw);
    let token_address = Bits256(token_bytes);
    
    println!("Registering token on Fuel...");
    
    let sub_id = registry
        .methods()
        .register_asset(
            origin_chain_id as u64,
            token_address,
            9, 
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
  
    
    let remote_wallet = get_evm_wallet().await;
    let contracts = SepoliaContracts::initialize(remote_wallet.clone()).await;
    let remote_wr = contracts.warp_route_collateral; 
    let remote_mailbox = contracts.mailbox;
    
    let fuel_domain = get_fuel_domain();
    let fuel_registry_bytes = FixedBytes::from_slice(registry_id.as_slice());
    
    
    let remote_wr_address = load_remote_wr_addresses("CTR").unwrap(); // Collateral token router
    let remote_wr_hex = hex::decode(remote_wr_address.strip_prefix("0x").unwrap()).unwrap();
    
    let mut remote_wr_array = [0u8; 32];
    remote_wr_array[12..].copy_from_slice(&remote_wr_hex);
    
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
    
    
    
    let amount = 1_000_000u64;
    
    let recipient = FixedBytes::from_slice(wallet.address().hash().as_slice());
    
    let quote_dispatch = remote_wr
        .quoteGasPayment(fuel_domain)
        .call()
        .await
        .unwrap()
        ._0;
    
    let _ = remote_wr
        .transferRemote_1(fuel_domain, recipient, U256::from(amount))
        .value(quote_dispatch + U256::from(amount)) // Pay for gas + token amount
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .map_err(|e| format!("Failed to transfer remote: {:?}", e))?;
    
    
    let msg_id = remote_mailbox.latestDispatchedId().call().await.unwrap()._0;
    
    if FixedBytes::const_is_zero(&msg_id) {
        return Err("Failed to get valid message ID".to_string());
    }
    
    println!("✓ Got message ID: {:?}", msg_id);
    

    let delivery_success = monitor_fuel_for_delivery(mailbox.clone(), msg_id).await;
    assert!(delivery_success, "Message was not delivered to Fuel");
    println!("✓ Message was delivered to Fuel");
    

    let wrapped_asset_id = minter.contract_id().asset_id(&sub_id);
    let redemption_ticket_asset_id = minter.contract_id().asset_id(&redemption_ticket_id.unwrap());
    
     let wrapped_balance = get_balance(
        wallet.provider().unwrap(),
        wallet.address(),
        wrapped_asset_id
    ).await.unwrap();
    
    let redemption_balance = get_balance(
        wallet.provider().unwrap(),
        wallet.address(),
        redemption_ticket_asset_id
    ).await.unwrap();
    
    assert_eq!(wrapped_balance, amount);
    assert_eq!(redemption_balance, amount);
    
    let total_supply = minter
        .methods()
        .total_supply(wrapped_asset_id)
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;
    
    assert_eq!(total_supply.unwrap_or(0), amount);
    
    println!("✓ Successfully minted wrapped tokens and redemption tickets");
    println!("✓ Wrapped balance: {}", wrapped_balance);
    println!("✓ Redemption ticket balance: {}", redemption_balance);
    
    
    println!("✅ wrapped_assets_bidirectional_flow test passed");
    Ok(start.elapsed().as_secs_f64())
}