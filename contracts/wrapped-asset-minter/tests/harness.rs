#[cfg(test)]
mod asset_minter {

    use fuels::{
        accounts::wallet::{WalletUnlocked, DEFAULT_DERIVATION_PATH_PREFIX},
        prelude::*,
        types::{coin::Coin, AssetId, Bits256, Bytes32, ContractId, Identity},
    };

    abigen!(
        Contract(
            name = "WrappedAssetMinter",
            abi = "contracts/wrapped-asset-minter/out/debug/wrapped-asset-minter-abi.json"
        ),
        Contract(
            name = "AssetRegistry",
            abi = "contracts/asset-registry/out/debug/asset-registry-abi.json"
        ),
        Contract(
            name = "MockHyperlaneMailbox",
            abi = "contracts/mocks/mock-mailbox/out/debug/mock-mailbox-abi.json"
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

    fn to_fuel_identity(wallet: &WalletUnlocked) -> Identity {
        Identity::Address(wallet.address().into())
    }

    async fn deploy_registry(
        wallet: &WalletUnlocked,
    ) -> (AssetRegistry<WalletUnlocked>, ContractId) {
        let id = Contract::load_from(
            "../asset-registry/out/debug/asset-registry.bin",
            LoadConfiguration::default(),
        )
        .unwrap()
        .deploy(wallet, TxPolicies::default())
        .await
        .unwrap();

        let instance = AssetRegistry::new(&id, wallet.clone());
        (instance, id.into())
    }

    async fn deploy_minter(
        wallet: &WalletUnlocked,
    ) -> (WrappedAssetMinter<WalletUnlocked>, ContractId) {
        let id = Contract::load_from(
            "./out/debug/wrapped-asset-minter.bin",
            LoadConfiguration::default(),
        )
        .unwrap()
        .deploy(wallet, TxPolicies::default())
        .await
        .unwrap();

        let instance = WrappedAssetMinter::new(&id, wallet.clone());
        (instance, id.into())
    }

    async fn deploy_mock_mailbox(
        wallet: &WalletUnlocked,
    ) -> (MockHyperlaneMailbox<WalletUnlocked>, ContractId) {
        let id = Contract::load_from(
            "/Users/praneshasp/Projects/fuel/fuel-hyp-2/contracts/mocks/mock-mailbox/out/debug/mock-mailbox.bin",
            LoadConfiguration::default(),
        )
        .unwrap()
        .deploy(wallet, TxPolicies::default())
        .await
        .unwrap();

        let instance = MockHyperlaneMailbox::new(&id, wallet.clone());
        (instance, id.into())
    }

    async fn setup_contracts() -> (
        WrappedAssetMinter<WalletUnlocked>,
        AssetRegistry<WalletUnlocked>,
        MockHyperlaneMailbox<WalletUnlocked>,
        ContractId,
        ContractId,
        ContractId,
        WalletUnlocked,
    ) {
        // Launch a local network and deploy the contracts
        let mut wallets = get_wallets().to_vec();
        let mut wallet = wallets.pop().unwrap();
        let coins = add_coins(&wallet);
        let provider = setup_test_provider(coins.clone(), vec![], None, None)
            .await
            .unwrap();
        let _ = &wallet.set_provider(provider);

        // Deploy all contracts
        let (registry_instance, registry_id) = deploy_registry(&wallet).await;
        let (minter_instance, minter_id) = deploy_minter(&wallet).await;
        let (mailbox_instance, mailbox_id) = deploy_mock_mailbox(&wallet).await;

        // Initialize registry with minter
        registry_instance
            .methods()
            .initialize(to_fuel_identity(&wallet), minter_id)
            .call()
            .await
            .unwrap();

        // Initialize minter with registry
        minter_instance
            .methods()
            .initialize(
                to_fuel_identity(&wallet),
                Bits256::from(AssetId::new(*registry_id)),
            )
            .call()
            .await
            .unwrap();

        // Initialize mailbox with registry
        mailbox_instance
            .methods()
            .initialize(Bits256::from(AssetId::new(*registry_id)))
            .call()
            .await
            .unwrap();

        (
            minter_instance,
            registry_instance,
            mailbox_instance,
            minter_id,
            registry_id,
            mailbox_id,
            wallet,
        )
    }

    #[tokio::test]
    async fn can_deploy_contracts() {
        let (_, _, _, minter_id, registry_id, mailbox_id, _) = setup_contracts().await;
        assert_ne!(minter_id, ContractId::from([0u8; 32]));
        assert_ne!(registry_id, ContractId::from([0u8; 32]));
        assert_ne!(mailbox_id, ContractId::from([0u8; 32]));
    }

    #[tokio::test]
    async fn test_minter_initialization() {
        let (minter, _, _, _, _, _, _) = setup_contracts().await;
        let total = minter.methods().total_assets().call().await.unwrap().value;
        assert_eq!(total, 0);

        let owner_state = minter.methods().owner().call().await.unwrap().value;
        let mut wallets = get_wallets().to_vec();
        let owner = wallets.pop().unwrap();
        assert!(owner_state == State::Initialized(to_fuel_identity(&owner)));
    }

    #[tokio::test]
    async fn test_transfer_ownership() {
        let (minter, _, _, minter_id, _, _, wallet) = setup_contracts().await;
        let wallets = get_wallets();

        let new_owner = to_fuel_identity(&wallets[2]);
        let result = minter.methods().transfer_ownership(new_owner).call().await;

        assert!(result.is_ok());

        let non_owner_instance = WrappedAssetMinter::new(minter_id, wallets[3].clone());

        let newer_owner = to_fuel_identity(&wallets[4]);
        let result = non_owner_instance
            .methods()
            .transfer_ownership(newer_owner)
            .call()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mint() {
        let (minter, registry, _, _, _, mailbox_id, wallet) = setup_contracts().await;
        let wallets = get_wallets();

        // Register a bridge
        let bridge_name = "TestBridge".to_string();
        let bridge_address = Identity::ContractId(mailbox_id);
        let bridge_id = registry
            .methods()
            .register_bridge(bridge_name, bridge_address)
            .call()
            .await
            .unwrap()
            .value;

        // Register an asset
        let chain_id = 1u64; // Ethereum
        let token_address = Bits256::from(AssetId::from([5u8; 32]));
        let decimals = 6;
        let name = "Wrapped USDC".to_string();
        let symbol = "wUSDC".to_string();

        let sub_id = registry
            .methods()
            .register_asset(chain_id, token_address, decimals, name, symbol)
            .call()
            .await
            .unwrap()
            .value;

        // Authorize the bridge for this asset
        registry
            .methods()
            .authorize_bridge_for_asset(bridge_id, sub_id)
            .call()
            .await
            .unwrap();

        // Mint some tokens
        let recipient = to_fuel_identity(&wallet);
        let amount = 1000; // 1 USDC with 6 decimals

        let result = minter
            .methods()
            .mint(bridge_id, recipient, sub_id, amount)
            .with_contract_ids(&[registry.contract_id().clone()])
            .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
            .call()
            .await
            .unwrap();

        // Convert the result to a hexadecimal string if needed
        let asset_id = minter.clone().contract_id().asset_id(&sub_id);

        let total_supply = minter
            .methods()
            .total_supply(asset_id)
            .call()
            .await
            .unwrap()
            .value;
        println!("total_supply: {:?}", total_supply);
        assert!(total_supply.is_some());
        assert_eq!(total_supply.unwrap(), amount);

        // Check total assets increased
        let total_assets = minter.methods().total_assets().call().await.unwrap().value;

        assert_eq!(total_assets, 1);

        // Test token metadata
        let token_name = minter
            .methods()
            .name(asset_id)
            .with_contract_ids(&[registry.contract_id().clone()])
            .call()
            .await
            .unwrap()
            .value;

        assert!(token_name.is_some());
        assert_eq!(token_name.unwrap(), "Wrapped USDC");

        let token_symbol = minter
            .methods()
            .symbol(asset_id)
            .with_contract_ids(&[registry.contract_id().clone()])
            .call()
            .await
            .unwrap()
            .value;

        assert!(token_symbol.is_some());
        assert_eq!(token_symbol.unwrap(), "wUSDC");

        let token_decimals = minter
            .methods()
            .decimals(asset_id)
            .with_contract_ids(&[registry.contract_id().clone()])
            .call()
            .await
            .unwrap()
            .value;

        assert!(token_decimals.is_some());
        assert_eq!(token_decimals.unwrap(), 6);
    }

    #[tokio::test]
    async fn test_metadata() {
        let (minter, registry, _, _, _, mailbox_id, wallet) = setup_contracts().await;

        let bridge_name = "TestBridge".to_string();
        let bridge_address = Identity::ContractId(mailbox_id);
        let bridge_id = registry
            .methods()
            .register_bridge(bridge_name, bridge_address)
            .call()
            .await
            .unwrap()
            .value;

        let chain_id = 1u64; // Ethereum
        let token_address = Bits256::from(AssetId::from([6u8; 32]));
        let decimals = 18;
        let name = "UniWrapped ETH".to_string();
        let symbol = "uwETH".to_string();

        let sub_id = registry
            .methods()
            .register_asset(chain_id, token_address, decimals, name, symbol)
            .call()
            .await
            .unwrap()
            .value;

        registry
            .methods()
            .authorize_bridge_for_asset(bridge_id, sub_id)
            .call()
            .await
            .unwrap();

        let recipient = to_fuel_identity(&wallet);
        let amount = 1_000_000_000_000_000_000; // 1 ETH with 18 decimals

        minter
            .methods()
            .mint(bridge_id, recipient, sub_id, amount)
            .with_contract_ids(&[registry.contract_id().clone()])
            .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
            .call()
            .await
            .unwrap();

        let asset_id = minter.clone().contract_id().asset_id(&sub_id);

        // Test metadata
        // 1. bridged:chain
        let chain_metadata = minter
            .methods()
            .metadata(asset_id, "bridged:chain".to_string())
            .with_contract_ids(&[registry.contract_id().clone()])
            .call()
            .await
            .unwrap()
            .value;

        assert!(chain_metadata.is_some());

        // 2. bridged:address
        let address_metadata = minter
            .methods()
            .metadata(asset_id, "bridged:address".to_string())
            .with_contract_ids(&[registry.contract_id().clone()])
            .call()
            .await
            .unwrap()
            .value;

        assert!(address_metadata.is_some());

        // 3. bridged:decimals
        let decimals_metadata = minter
            .methods()
            .metadata(asset_id, "bridged:decimals".to_string())
            .with_contract_ids(&[registry.contract_id().clone()])
            .call()
            .await
            .unwrap()
            .value;

        assert!(decimals_metadata.is_some());

        // 4. Non-existent metadata key
        let nonexistent_metadata = minter
            .methods()
            .metadata(asset_id, "non:existent".to_string())
            .with_contract_ids(&[registry.contract_id().clone()])
            .call()
            .await
            .unwrap()
            .value;

        assert!(nonexistent_metadata.is_none());
    }

    #[tokio::test]
    async fn test_unauthorized_mint() {
        let (minter, registry, _, _, _, mailbox_id, wallet) = setup_contracts().await;

        let bridge_name = "UnauthorizedBridge".to_string();
        let bridge_address = Identity::ContractId(mailbox_id);

        let bridge_id = registry
            .methods()
            .register_bridge(bridge_name, bridge_address)
            .call()
            .await
            .unwrap()
            .value;

        let chain_id = 1u64;
        let token_address = Bits256::from(AssetId::from([7u8; 32]));
        let decimals = 8;
        let name = "Unauthorized Test".to_string();
        let symbol = "UNAUTH".to_string();

        let sub_id = registry
            .methods()
            .register_asset(chain_id, token_address, decimals, name, symbol)
            .call()
            .await
            .unwrap()
            .value;

        // Try to mint without authorization - should fail
        let recipient = to_fuel_identity(&wallet);
        let amount = 1_000_000;

        let result = minter
            .methods()
            .mint(bridge_id, recipient, sub_id, amount)
            .with_contract_ids(&[registry.contract_id().clone()])
            .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
            .call()
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Not authorized minter") || error.contains("NotAuthorizedMinter"));
    }
}
