use std::str::FromStr;

use fuels::types::{AssetId, Bytes};
use fuels::{
    accounts::wallet::{WalletUnlocked, DEFAULT_DERIVATION_PATH_PREFIX},
    prelude::*,
    types::{bech32::FUEL_BECH32_HRP, coin::Coin, Bits256, ContractId, Identity},
};
use hyperlane_core::{Encode, HyperlaneMessage as HyperlaneAgentMessage, H256};

abigen!(
    Contract(
        name = "AssetRegistry",
        abi = "contracts/asset-registry/out/debug/asset-registry-abi.json"
    ),
    Contract(
        name = "MockHyperlaneMailbox",
        abi = "contracts/mocks/mock-mailbox/out/debug/mock-mailbox-abi.json"
    ),
    Contract(
        name = "WrappedAssetMinter",
        abi = "contracts/wrapped-asset-minter/out/debug/wrapped-asset-minter-abi.json"
    ),
    Contract(
        name = "Mailbox",
        abi = "contracts/mailbox/out/debug/mailbox-abi.json"
    ),
    Contract(
        name = "TestInterchainSecurityModule",
        abi = "contracts/test/ism-test/out/debug/ism-test-abi.json"
    )
);

pub fn get_wallets() -> [WalletUnlocked; 20] {
    let phrase = "test test test test test test test test test test test junk";
    let mut wallets: Vec<WalletUnlocked> = vec![];

    for index in 0u32..20u32 {
        let path = format!("{DEFAULT_DERIVATION_PATH_PREFIX}/0'/0/{index}");
        let wallet =
            WalletUnlocked::new_from_mnemonic_phrase_with_path(phrase, None, &path).unwrap();
        wallets.push(wallet.clone());
    }
    wallets.try_into().unwrap()
}

pub fn add_coins(wallet: &WalletUnlocked) -> Vec<Coin> {
    let mut coins: Vec<Coin> = vec![];

    let assets: Vec<AssetId> = (0u8..3).map(|i| AssetId::new([i; 32])).collect();

    for asset_id in assets.clone().iter() {
        let mut coin =
            setup_single_asset_coins(wallet.address(), asset_id.clone(), 10, 1_000_000_000_000);
        coins.append(&mut coin);
    }
    coins
}

async fn get_contract_instances() -> (
    AssetRegistry<WalletUnlocked>,
    MockHyperlaneMailbox<WalletUnlocked>,
    ContractId,
    ContractId,
    Provider,
    WalletUnlocked,
) {
    // Launch a local network and deploy the contract
    let mut wallets = get_wallets().to_vec();
    let mut wallet = wallets.pop().unwrap();
    let coins = add_coins(&wallet);
    let provider = setup_test_provider(coins.clone(), vec![], None, None)
        .await
        .unwrap();
    let _ = &wallet.set_provider(provider.clone());

    // Deploy Asset Registry
    let registry_id = Contract::load_from(
        "./out/debug/asset-registry.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&wallet, TxPolicies::default())
    .await
    .unwrap();

    let registry_instance = AssetRegistry::new(&registry_id, wallet.clone());

    // Deploy Mock Hyperlane Mailbox
    let mailbox_id = Contract::load_from(
        "/Users/praneshasp/Projects/fuel/fuel-hyp-2/contracts/mocks/mock-mailbox/out/debug/mock-mailbox.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&wallet, TxPolicies::default())
    .await
    .unwrap();

    println!("Admin Wallet: {}", wallet.address());

    let mailbox_instance = MockHyperlaneMailbox::new(&mailbox_id, wallet.clone());

    (
        registry_instance,
        mailbox_instance,
        registry_id.into(),
        mailbox_id.into(),
        provider,
        wallet,
    )
}

fn to_fuel_identity(wallet: &WalletUnlocked) -> Identity {
    Identity::Address(wallet.address().into())
}

#[tokio::test]
async fn can_get_contract_id() {
    let (_, _, registry_id, mailbox_id, _, _) = get_contract_instances().await;
    assert_ne!(registry_id, ContractId::from([0u8; 32]));
    assert_ne!(mailbox_id, ContractId::from([0u8; 32]));
}

#[tokio::test]
async fn test_initialize() {
    let (registry_instance, mailbox_instance, registry_id, _, _, _) =
        get_contract_instances().await;
    let wallets = get_wallets();

    // Initialize registry with a dummy minter contract
    let minter_contract = ContractId::from([1u8; 32]);
    let result = registry_instance
        .methods()
        .initialize(to_fuel_identity(&wallets[1]), minter_contract)
        .call()
        .await;
    assert!(result.is_ok());

    // Initialize the mailbox with the registry contract
    let result = mailbox_instance
        .methods()
        .initialize(Bits256::from(AssetId::from(*registry_id)))
        .call()
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_owner() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let new_owner = to_fuel_identity(&wallets[2]);
    let result = instance.methods().update_owner(new_owner).call().await;

    assert!(result.is_ok());

    // Create a new instance with a different wallet (non-owner)
    let non_owner_instance = AssetRegistry::new(instance.contract_id(), wallets[3].clone());

    // Try updating owner with a non-owner wallet, should fail
    let newer_owner = to_fuel_identity(&wallets[4]);
    let result = non_owner_instance
        .methods()
        .update_owner(newer_owner)
        .call()
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_minter_contract() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let new_minter_contract = ContractId::from([2u8; 32]);
    let result = instance
        .methods()
        .update_minter_contract(new_minter_contract)
        .call()
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_register_bridge() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let bridge_name = "HyperlaneBridge".to_string();
    // Use a wallet address as the bridge contract address
    let bridge_address = to_fuel_identity(&wallets[5]);

    let result = instance
        .methods()
        .register_bridge(bridge_name, bridge_address)
        .call()
        .await;

    assert!(result.is_ok());

    let bridge_id = result.unwrap().value;
    assert_ne!(bridge_id, Bits256::from(AssetId::from([0u8; 32])));
}

#[tokio::test]
async fn test_register_asset() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let chain_id = 1u64;
    let token_address = Bits256::from(AssetId::from([3u8; 32]));
    let decimals = 6;
    let name = "UniWrapped USDC".to_string();
    let symbol = "uwUSDC".to_string();

    let result = instance
        .methods()
        .register_asset(chain_id, token_address, decimals, name, symbol)
        .call()
        .await;

    assert!(result.is_ok());

    let sub_id = result.unwrap().value;
    assert_ne!(sub_id, Bits256::from(AssetId::from([0u8; 32])));
}

#[tokio::test]
async fn test_is_asset_registered() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let chain_id = 1u64; // Ethereum
    let token_address = Bits256::from(AssetId::from([4u8; 32]));
    let decimals = 18;
    let name = "UniWrapped Ethereum".to_string();
    let symbol = "uwETH".to_string();

    let result = instance
        .methods()
        .register_asset(chain_id, token_address, decimals, name, symbol)
        .call()
        .await
        .unwrap();

    let sub_id = result.value;

    let is_registered = instance
        .methods()
        .is_asset_registered(sub_id)
        .call()
        .await
        .unwrap();

    assert!(is_registered.value);

    let fake_sub_id = Bits256::from(AssetId::from([99u8; 32]));
    let is_registered = instance
        .methods()
        .is_asset_registered(fake_sub_id)
        .call()
        .await
        .unwrap();

    assert!(!is_registered.value);
}

#[tokio::test]
async fn test_get_asset_params() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let chain_id = 1u64; // eth chain id
    let token_address = Bits256::from(AssetId::from([5u8; 32]));
    let decimals = 9;
    let name = "Test Token".to_string();
    let symbol = "TEST".to_string();

    let result = instance
        .methods()
        .register_asset(chain_id, token_address, decimals, name, symbol)
        .call()
        .await
        .unwrap();

    let sub_id = result.value;

    let params = instance
        .methods()
        .get_asset_params(sub_id)
        .call()
        .await
        .unwrap();

    assert!(params.value.is_some());

    let params = params.value.unwrap();

    assert_eq!(params.sub_id, sub_id);
    assert_eq!(params.origin_chain_id, chain_id);
    assert_eq!(params.origin_token_address, token_address);
    assert_eq!(params.origin_decimals, decimals);

    let details = instance
        .methods()
        .get_token_details(sub_id)
        .call()
        .await
        .unwrap();

    assert!(details.value.is_some());
    let details = details.value.unwrap();
    assert_eq!(details.name, "Test Token");
    assert_eq!(details.symbol, "TEST");
    assert_eq!(details.decimals, 9);

    let fake_sub_id = Bits256::from(AssetId::from([99u8; 32]));
    let params = instance
        .methods()
        .get_asset_params(fake_sub_id)
        .call()
        .await
        .unwrap();

    assert!(params.value.is_none());
}

#[tokio::test]
async fn test_bridge_authorization_for_asset() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    // Register a bridge with a bridge contract address
    let bridge_name = "AuthTest".to_string();
    let bridge_address = to_fuel_identity(&wallets[6]);
    let bridge_id = instance
        .methods()
        .register_bridge(bridge_name, bridge_address)
        .call()
        .await
        .unwrap()
        .value;

    // Register an asset
    let chain_id = 1u64;
    let token_address = Bits256::from(AssetId::from([6u8; 32]));
    let decimals = 8;
    let name = "Test Authorized".to_string();
    let symbol = "AUTH".to_string();

    let sub_id = instance
        .methods()
        .register_asset(chain_id, token_address, decimals, name, symbol)
        .call()
        .await
        .unwrap()
        .value;

    let is_bridge_registered = instance
        .methods()
        .is_bridge_registered(bridge_id)
        .call()
        .await
        .unwrap();

    assert!(is_bridge_registered.value);

    let is_authorized = instance
        .methods()
        .is_bridge_authorized_for_asset(bridge_id, sub_id)
        .call()
        .await
        .unwrap();

    assert!(!is_authorized.value);

    let result = instance
        .methods()
        .authorize_bridge_for_asset(bridge_id, sub_id)
        .call()
        .await;

    assert!(result.is_ok());

    let is_authorized = instance
        .methods()
        .is_bridge_authorized_for_asset(bridge_id, sub_id)
        .call()
        .await
        .unwrap();

    assert!(is_authorized.value);

    let result = instance
        .methods()
        .deauthorize_bridge_for_asset(bridge_id, sub_id)
        .call()
        .await;

    assert!(result.is_ok());

    let is_authorized = instance
        .methods()
        .is_bridge_authorized_for_asset(bridge_id, sub_id)
        .call()
        .await
        .unwrap();

    assert!(!is_authorized.value);
}

#[tokio::test]
async fn test_register_duplicate_asset() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let chain_id = 1u64;
    let token_address = Bits256::from(AssetId::from([7u8; 32]));
    let decimals = 18;
    let name = "Duplicate".to_string();
    let symbol = "DUP".to_string();

    let result = instance
        .methods()
        .register_asset(
            chain_id,
            token_address,
            decimals,
            name.clone(),
            symbol.clone(),
        )
        .call()
        .await;

    assert!(result.is_ok());

    let result = instance
        .methods()
        .register_asset(chain_id, token_address, decimals, name, symbol)
        .call()
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("already registered"));
}

#[tokio::test]
async fn test_sub_id_consistency() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    // Register two assets with the same token address but different chain IDs
    let token_address = Bits256::from(AssetId::from([8u8; 32]));
    let decimals = 18;

    // Register on Ethereum (chain_id = 1)
    let chain_id_eth = 1u64;
    let sub_id_eth = instance
        .methods()
        .register_asset(
            chain_id_eth,
            token_address,
            decimals,
            "UniWrapped ETH".to_string(),
            "uwETH".to_string(),
        )
        .call()
        .await
        .unwrap()
        .value;

    let chain_id_bsc = 56u64;
    let sub_id_bsc = instance
        .methods()
        .register_asset(
            chain_id_bsc,
            token_address,
            decimals,
            "UniWrapped BNB".to_string(),
            "uwBNB".to_string(),
        )
        .call()
        .await
        .unwrap()
        .value;

    assert_ne!(sub_id_eth, sub_id_bsc);

    let is_eth_registered = instance
        .methods()
        .is_asset_registered(sub_id_eth)
        .call()
        .await
        .unwrap();

    let is_bsc_registered = instance
        .methods()
        .is_asset_registered(sub_id_bsc)
        .call()
        .await
        .unwrap();

    assert!(is_eth_registered.value);
    assert!(is_bsc_registered.value);
}

#[tokio::test]
async fn test_bridge_is_registered() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let bridge_name = "TestBridge".to_string();
    let bridge_address = to_fuel_identity(&wallets[7]);
    let bridge_id = instance
        .methods()
        .register_bridge(bridge_name, bridge_address)
        .call()
        .await
        .unwrap()
        .value;

    let is_registered = instance
        .methods()
        .is_bridge_registered(bridge_id)
        .call()
        .await
        .unwrap();

    assert!(is_registered.value);

    let fake_bridge_id = Bits256::from(AssetId::from([99u8; 32]));
    let is_registered = instance
        .methods()
        .is_bridge_registered(fake_bridge_id)
        .call()
        .await
        .unwrap();

    assert!(!is_registered.value);
}

// #[tokio::test]
// async fn test_mock_hyperlane_integration() {
//     let (registry_instance, mailbox_instance, registry_id, _, _) = get_contract_instances().await;
//     let wallets = get_wallets();

//     let minter_contract = ContractId::from([1u8; 32]);
//     registry_instance
//         .methods()
//         .initialize(to_fuel_identity(&wallets[0]), minter_contract)
//         .call()
//         .await
//         .unwrap();

//     mailbox_instance
//         .methods()
//         .initialize(Bits256::from(AssetId::from(*registry_id)))
//         .call()
//         .await
//         .unwrap();

//     let bridge_name = "HyperlaneBridge".to_string();
//     let bridge_address = Identity::ContractId(mailbox_instance.contract_id().clone().into());

//     let register_result = registry_instance
//         .methods()
//         .register_bridge(bridge_name, bridge_address)
//         .call()
//         .await
//         .unwrap();

//     let bridge_id = register_result.value;

//     let is_registered = registry_instance
//         .methods()
//         .is_bridge_registered(bridge_id)
//         .call()
//         .await
//         .unwrap();

//     assert!(is_registered.value);
// }

#[tokio::test]
async fn test_redemption_ticket_registration() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let chain_id = 1u64;
    let token_address = Bits256::from(AssetId::from([10u8; 32]));
    let decimals = 18;
    let name = "UniWrapped Token".to_string();
    let symbol = "uwTEST".to_string();

    let result = instance
        .methods()
        .register_asset(chain_id, token_address, decimals, name, symbol)
        .call()
        .await
        .unwrap();

    let asset_sub_id = result.value;

    let redemption_ticket_id = instance
        .methods()
        .get_redemption_ticket_for_sub_id(asset_sub_id)
        .call()
        .await
        .unwrap()
        .value;

    assert!(redemption_ticket_id.is_some());
    let redemption_ticket_id = redemption_ticket_id.unwrap();

    let is_redemption_ticket = instance
        .methods()
        .is_redemption_ticket(redemption_ticket_id)
        .call()
        .await
        .unwrap()
        .value;

    assert!(is_redemption_ticket);

    let original_asset = instance
        .methods()
        .get_sub_id_for_redemption_ticket(redemption_ticket_id)
        .call()
        .await
        .unwrap()
        .value;

    assert!(original_asset.is_some());
    assert_eq!(original_asset.unwrap(), asset_sub_id);
}

#[tokio::test]
async fn test_redemption_ticket_metadata() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let chain_id = 1u64;
    let token_address = Bits256::from(AssetId::from([11u8; 32]));
    let decimals = 18;
    let name = "UniWrapped Token".to_string();
    let symbol = "uwEXT".to_string();

    let result = instance
        .methods()
        .register_asset(chain_id, token_address, decimals, name, symbol)
        .call()
        .await
        .unwrap();

    let asset_sub_id = result.value;

    let redemption_ticket_id = instance
        .methods()
        .get_redemption_ticket_for_sub_id(asset_sub_id)
        .call()
        .await
        .unwrap()
        .value
        .unwrap();

    let token_details = instance
        .methods()
        .get_token_details(redemption_ticket_id)
        .call()
        .await
        .unwrap()
        .value;

    assert!(token_details.is_some());
    let details = token_details.unwrap();

    assert_eq!(details.name, "Redemption Ticket");
    assert_eq!(details.symbol, "RT");
    assert_eq!(details.decimals, 9);
}

#[tokio::test]
async fn test_router_functions() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let test_domain = 1717982312u32;
    let test_router =
        Bits256::from_hex_str("0x00000000000000000000000000000000000000000000000000000000deadbeef")
            .unwrap();

    let router_before = instance
        .methods()
        .router(test_domain)
        .call()
        .await
        .unwrap()
        .value;

    assert_eq!(router_before, Bits256::zeroed());

    let enroll_result = instance
        .methods()
        .enroll_remote_router(test_domain, test_router)
        .call()
        .await;

    assert!(enroll_result.is_ok());

    let router_after = instance
        .methods()
        .router(test_domain)
        .call()
        .await
        .unwrap()
        .value;

    assert_eq!(router_after, test_router);

    // Test unenroll_remote_router
    let unenroll_result = instance
        .methods()
        .unenroll_remote_router(test_domain)
        .call()
        .await
        .unwrap()
        .value;

    assert!(unenroll_result);

    // Verify the router is removed
    let router_after_unenroll = instance
        .methods()
        .router(test_domain)
        .call()
        .await
        .unwrap()
        .value;

    assert_eq!(router_after_unenroll, Bits256::zeroed());
}

#[tokio::test]
async fn test_router_decimals() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let test_router =
        Bits256::from_hex_str("0x00000000000000000000000000000000000000000000000000000000deadbeef")
            .unwrap();
    let test_decimals = 18u8;

    let set_decimals_result = instance
        .methods()
        .set_remote_router_decimals(test_router, test_decimals)
        .call()
        .await;

    assert!(set_decimals_result.is_ok());

    let decimals = instance
        .methods()
        .remote_router_decimals(test_router)
        .call()
        .await
        .unwrap()
        .value;

    assert_eq!(decimals, test_decimals);
}

#[tokio::test]
async fn test_batch_enroll_routers() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let test_domains = vec![1u32, 56u32, 137u32]; // Ethereum, BSC, Polygon
    let test_routers = vec![
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000001")
            .unwrap(),
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000056")
            .unwrap(),
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000137")
            .unwrap(),
    ];

    let enroll_result = instance
        .methods()
        .enroll_remote_routers(test_domains.clone(), test_routers.clone())
        .call()
        .await;

    assert!(enroll_result.is_ok());

    for (i, domain) in test_domains.iter().enumerate() {
        let router = instance
            .methods()
            .router(*domain)
            .call()
            .await
            .unwrap()
            .value;

        assert_eq!(
            router, test_routers[i],
            "Router was not set correctly for domain {}",
            domain
        );
    }
}

#[tokio::test]
async fn test_get_all_domains_and_routers() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let test_domains = vec![1u32, 56u32, 137u32]; // Ethereum, BSC, Polygon
    let test_routers = vec![
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000001")
            .unwrap(),
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000056")
            .unwrap(),
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000137")
            .unwrap(),
    ];

    for (i, domain) in test_domains.iter().enumerate() {
        instance
            .methods()
            .enroll_remote_router(*domain, test_routers[i])
            .call()
            .await
            .unwrap();
    }

    let all_domains = instance.methods().all_domains().call().await.unwrap().value;

    assert_eq!(all_domains.len(), test_domains.len());
    for domain in test_domains.iter() {
        assert!(
            all_domains.contains(domain),
            "Domain {} not found in all_domains",
            domain
        );
    }

    let all_routers = instance.methods().all_routers().call().await.unwrap().value;

    assert_eq!(all_routers.len(), test_routers.len());
    for router in test_routers.iter() {
        assert!(
            all_routers.contains(router),
            "Router {:?} not found in all_routers",
            router
        );
    }
}

#[tokio::test]
async fn test_router_length_mismatch() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let test_domains = vec![1u32, 56u32, 137u32]; // Ethereum, BSC, Polygon
    let test_routers = vec![
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000001")
            .unwrap(),
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000056")
            .unwrap(),
    ];

    let enroll_result = instance
        .methods()
        .enroll_remote_routers(test_domains, test_routers)
        .call()
        .await;

    assert!(enroll_result.is_err());
    let error = enroll_result.unwrap_err().to_string();
    assert!(
        error.contains("RouterLengthMismatch") || error.contains("length mismatch"),
        "Error should indicate length mismatch: {}",
        error
    );
}

#[tokio::test]
async fn test_unauthorized_router_operations() {
    let (instance, _, _, _, _, _) = get_contract_instances().await;
    let wallets = get_wallets();

    let minter_contract = ContractId::from([1u8; 32]);
    instance
        .methods()
        .initialize(to_fuel_identity(&wallets[0]), minter_contract)
        .call()
        .await
        .unwrap();

    let non_owner_instance = AssetRegistry::new(instance.contract_id(), wallets[3].clone());

    let test_domain = 123u32;
    let test_router =
        Bits256::from_hex_str("0x0000000000000000000000000000000000000000000000000000000000000123")
            .unwrap();

    let enroll_result = non_owner_instance
        .methods()
        .enroll_remote_router(test_domain, test_router)
        .call()
        .await;

    assert!(enroll_result.is_err());

    let set_decimals_result = non_owner_instance
        .methods()
        .set_remote_router_decimals(test_router, 18u8)
        .call()
        .await;

    assert!(set_decimals_result.is_err());

    let unenroll_result = non_owner_instance
        .methods()
        .unenroll_remote_router(test_domain)
        .call()
        .await;

    assert!(unenroll_result.is_err());
}

#[tokio::test]
async fn test_mint_with_real_mailbox() {
    let (registry, _, registry_id, _, provider, admin_wallet) = get_contract_instances().await;

    let wallets = get_wallets();
    let mut user_wallet = wallets[10].clone();
    println!("Admin Wallet: {}", admin_wallet.address());

    let user_coins = add_coins(&user_wallet);

    user_wallet.set_provider(provider.clone());

    let minter_id = Contract::load_from(
        "../wrapped-asset-minter/out/debug/wrapped-asset-minter.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&admin_wallet, TxPolicies::default())
    .await
    .unwrap();

    let minter = WrappedAssetMinter::new(&minter_id, admin_wallet.clone());

    println!("REGISTRY ID: {:?}", registry_id);
    println!("MINTER ID: {:?}", minter_id);

    registry
        .methods()
        .update_minter_contract(minter_id.clone())
        .call()
        .await;

    minter
        .methods()
        .initialize(
            to_fuel_identity(&admin_wallet),
            Bits256::from(AssetId::new(*registry_id)),
        )
        .call()
        .await;

    let mailbox_id = Contract::load_from(
        "../mailbox/out/debug/mailbox.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&admin_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!("MAILBOX ID: {:?}", mailbox_id);

    let mailbox = Mailbox::new(&mailbox_id, admin_wallet.clone());

    let ism_id = Contract::load_from(
        "../test/ism-test/out/debug/ism-test.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&admin_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!("ISM ID: {:?}", ism_id);

    let hook_id = Contract::load_from(
        "../mocks/mock-post-dispatch/out/debug/mock-post-dispatch.bin",
        LoadConfiguration::default(),
    )
    .unwrap()
    .deploy(&admin_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!("HOOK ID: {:?}", hook_id);

    let owner_identity = Identity::Address(admin_wallet.address().into());
    let _ = mailbox
        .methods()
        .initialize(
            owner_identity,
            Bits256(ContractId::from(ism_id.clone()).into()),
            Bits256(ContractId::from(hook_id.clone()).into()),
            Bits256(ContractId::from(hook_id.clone()).into()),
        )
        .call()
        .await;

    let bridge_name = "RealHyperlaneBridge".to_string();
    let bridge_address = Identity::ContractId(mailbox_id.clone().into());

    let bridge_result = registry
        .methods()
        .register_bridge(bridge_name, bridge_address)
        .call()
        .await;

    let bridge_id = bridge_result.unwrap().value;
    println!("BRIDGE ID: {:?}", bridge_id);

    let origin_chain_id = 1u64; // Ethereum
    let token_address = Bits256::from(AssetId::from([20u8; 32])); // Simulated token address
    let decimals = 9;
    let name = "Wrapped Ethereum".to_string();
    let symbol = "WETH".to_string();

    let asset_register_result = registry
        .methods()
        .register_asset(origin_chain_id, token_address, decimals, name, symbol)
        .call()
        .await;

    let asset_sub_id = asset_register_result.unwrap().value;
    println!("ASSET SUB ID: {:?}", asset_sub_id);

    let auth_result = registry
        .methods()
        .authorize_bridge_for_asset(bridge_id, asset_sub_id)
        .call()
        .await;

    let redemption_result = registry
        .methods()
        .get_redemption_ticket_for_sub_id(asset_sub_id)
        .call()
        .await;

    let redemption_ticket_id = redemption_result.unwrap().value.unwrap();
    println!("REDEMPTION TICKET ID: {:?}", redemption_ticket_id);

    let recipient_address = Bits256(admin_wallet.address().hash().into());
    let mint_amount = 1_000_000u64;
    println!("RECIPIENT ADDRESS: {:?}", recipient_address);

    let binding = registry.clone();
    let registry_contract_id = binding.contract_id();
    println!("REGISTRY CONTRACT ID: {:?}", registry_contract_id);

    let mut body = Vec::new();
    body.extend_from_slice(&recipient_address.0);

    let amount_bytes = mint_amount.to_be_bytes();
    let mut padded_amount = vec![0u8; 32 - amount_bytes.len()];
    padded_amount.extend_from_slice(&amount_bytes);
    body.extend_from_slice(&padded_amount);

    let registry_recipient = H256::from_slice(registry_contract_id.hash().as_slice());
    println!("REGISTRY RECIPIENT: {:?}", registry_recipient);

    let message = HyperlaneAgentMessage {
        version: 3u8,
        nonce: 0u32,
        origin: origin_chain_id as u32,
        sender: H256::from_slice(&token_address.0),
        destination: 0x6675656cu32,
        recipient: registry_recipient,
        body,
    };

    let test_ism = TestInterchainSecurityModule::new(&ism_id, admin_wallet.clone());
    let ism_accept_result = test_ism.methods().set_accept(true).call().await;

    let message_bytes = Bytes(message.to_vec());
    let metadata = Bytes(vec![0u8; 32]);

    let ism_from_mailbox = mailbox
        .methods()
        .default_ism()
        .simulate(Execution::StateReadOnly)
        .await
        .unwrap()
        .value;
    println!("ISM FROM MAILBOX: {:?}", ism_from_mailbox);

    let minter_from_registry = registry.methods().get_minter().call().await.unwrap().value;
    println!("MINTER FROM REGISTRY: {:?}", minter_from_registry);

    println!("CALL DETAILS:");
    println!("  Mailbox ID: {:?}", mailbox_id);
    println!("  Registry ID: {:?}", registry_contract_id);
    println!("  Minter ID: {:?}", minter_id);
    println!("  ISM ID: {:?}", ism_id);
    println!("  Hook ID: {:?}", hook_id);
    println!("  Message Recipient: {:?}", registry_recipient);
    println!("  Message Sender: {:?}", H256::from_slice(&token_address.0));

    let test_call = registry.methods().get_minter().call().await;

    println!("Attempting process call...");

    let process_result = mailbox
        .methods()
        .process(metadata, message_bytes)
        // .with_tx_policies(TxPolicies::default())
        // .determine_missing_contracts(Some(3))
        // .await.unwrap()
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        // .tx_params(tx_params)
        .with_contracts(&[&registry, &test_ism, &minter]) // &minter, &test_ism, &mailbox
        // .with_contract_ids(&[registry.contract_id().clone()]) // minter.clone().contract_id().clone(), mailbox_id.clone()
        .call()
        .await;

    println!(
        "Process result: {:?}",
        process_result.unwrap().decode_logs()
    );

    let asset_id = minter.contract_id().asset_id(&asset_sub_id);
    let redemption_ticket_asset_id = minter.contract_id().asset_id(&redemption_ticket_id);

    println!("Asset ID: {:?}", asset_id);
    println!(
        "Redemption ticket asset ID: {:?}",
        redemption_ticket_asset_id
    );

    // Check user's balance of wrapped asset
    let wrapped_asset_balance = provider
        .get_asset_balance(
            &Bech32Address::new(FUEL_BECH32_HRP, admin_wallet.address().hash()),
            asset_id,
        )
        .await
        .unwrap();

    assert_eq!(wrapped_asset_balance, mint_amount,);

    // Check user's balance of redemption ticket
    let redemption_ticket_balance = admin_wallet
        .get_asset_balance(&redemption_ticket_asset_id)
        .await
        .unwrap();

    assert_eq!(redemption_ticket_balance, mint_amount,);

    let total_supply = minter
        .methods()
        .total_supply(asset_id)
        .call()
        .await
        .unwrap()
        .value;

    assert_eq!(total_supply.unwrap(), mint_amount,);
}
