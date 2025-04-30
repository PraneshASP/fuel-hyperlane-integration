use std::collections::HashMap;

use fuels::{
    accounts::wallet::WalletUnlocked,
    programs::contract::Contract,
    types::{
        bech32::Bech32ContractId, transaction::TxPolicies, Bits256, ContractId, EvmAddress,
        Identity,
    },
};

use crate::{abis::*, get_deployment_config};

pub async fn deploy_mailbox(
    domain: u32,
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let binary_filepath = "../contracts/mailbox/out/debug/mailbox.bin";
    let configurables = MailboxConfigurables::default()
        .with_LOCAL_DOMAIN(domain)
        .unwrap()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();
    let mailbox_contract_id = Contract::load_from(
        binary_filepath,
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "mailbox: 0x{}",
        ContractId::from(mailbox_contract_id.clone())
    );

    mailbox_contract_id
}

pub async fn deploy_asset_registry(
    wallet: &WalletUnlocked,
    minter_id: ContractId,
) -> Bech32ContractId {
    let binary_filepath = "../contracts/asset-registry/out/debug/asset-registry.bin";
    let asset_registry_id = Contract::load_from(
        binary_filepath,
        get_deployment_config(),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    let asset_registry = AssetRegistry::new(asset_registry_id.clone(), wallet.clone());
    
    // Initialize asset registry with minter contract
    asset_registry
        .methods()
        .initialize(
            Identity::Address(wallet.address().into()),
            Bech32ContractId::from(minter_id),
        )
        .call()
        .await
        .unwrap();

    println!(
        "assetRegistry: 0x{}",
        ContractId::from(asset_registry_id.clone())
    );

    asset_registry_id
}

pub async fn deploy_aggregation_ism(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let configurables = AggregationISMConfigurables::default()
        .with_EXPECTED_INITIALIZER(wallet_bits)
        .unwrap();

    let aggregation_ism_id = Contract::load_from(
        "../contracts/ism/aggregation-ism/out/debug/aggregation-ism.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "AggregationISM: 0x{}",
        ContractId::from(aggregation_ism_id.clone())
    );

    aggregation_ism_id
}

pub async fn deploy_domain_routing_ism(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let configurables = DomainRoutingISMConfigurables::default()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let domain_routing_ism_id = Contract::load_from(
        "../contracts/ism/routing/domain-routing-ism/out/debug/domain-routing-ism.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "DomainRoutingISM: 0x{}",
        ContractId::from(domain_routing_ism_id.clone())
    );

    domain_routing_ism_id
}

pub async fn deploy_message_id_multisig_ism(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,

    threshold: u8,
) -> Bech32ContractId {
    let configurables = MessageIdMultisigISMConfigurables::default()
        .with_THRESHOLD(threshold)
        .unwrap()
        .with_EXPECTED_INITIALIZER(wallet_bits)
        .unwrap();

    let message_id_multisig_ism_id = Contract::load_from(
        "../contracts/ism/multisig/message-id-multisig-ism/out/debug/message-id-multisig-ism.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "Threshold {}/x MessageIdMultisigISM: 0x{}",
        threshold,
        ContractId::from(message_id_multisig_ism_id.clone())
    );

    message_id_multisig_ism_id
}

pub async fn deploy_mekle_root_multisig_ism(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,

    threshold: u8,
) -> Bech32ContractId {
    let configurables = MerkleRootMultisigISMConfigurables::default()
        .with_THRESHOLD(threshold)
        .unwrap()
        .with_EXPECTED_INITIALIZER(wallet_bits)
        .unwrap();

    let merkle_root_multisig_ism_id = Contract::load_from(
        "../contracts/ism/multisig/merkle-root-multisig-ism/out/debug/merkle-root-multisig-ism.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "Threshold {}/x MerkleRootMultisigISM: 0x{}",
        threshold,
        ContractId::from(merkle_root_multisig_ism_id.clone())
    );

    merkle_root_multisig_ism_id
}

pub async fn deploy_merkle_tree_hook(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let configurables = MerkleTreeHookConfigurables::default()
        .with_EXPECTED_INITIALIZER(wallet_bits)
        .unwrap();

    let merkle_tree_id = Contract::load_from(
        "../contracts/hooks/merkle-tree-hook/out/debug/merkle-tree-hook.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "merkleTreeHook: 0x{}",
        ContractId::from(merkle_tree_id.clone())
    );

    merkle_tree_id
}

pub async fn deploy_aggregation_hook(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let configurables = AggregationHookConfigurables::default()
        .with_EXPECTED_INITIALIZER(wallet_bits)
        .unwrap();

    let aggregation_hook_id = Contract::load_from(
        "../contracts/hooks/aggregation/out/debug/aggregation.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "aggregationHook: 0x{}",
        ContractId::from(aggregation_hook_id.clone())
    );

    aggregation_hook_id
}

pub async fn deploy_pausable_hook(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let configurables = PausableHookConfigurables::default()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let pausable_hook_id = Contract::load_from(
        "../contracts/hooks/pausable-hook/out/debug/pausable-hook.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "pausableHook: 0x{}",
        ContractId::from(pausable_hook_id.clone())
    );

    pausable_hook_id
}

pub async fn deploy_pausable_ism(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let configurables = PausableISMConfigurables::default()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let pausable_ism_id = Contract::load_from(
        "../contracts/ism/pausable-ism/out/debug/pausable-ism.bin",
        get_deployment_config().with_configurables(configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "PausableISM: 0x{}",
        ContractId::from(pausable_ism_id.clone())
    );

    pausable_ism_id
}

pub async fn deploy_protocol_fee_hook(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    const MAX_PROTOCOL_FEE: u64 = 1000000000; // From Base Mainnet Hook

    let protocol_fee_configurables = ProtocolFeeConfigurables::default()
        .with_MAX_PROTOCOL_FEE(MAX_PROTOCOL_FEE)
        .unwrap()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let protocol_fee_hook_id = Contract::load_from(
        "../contracts/hooks/protocol-fee/out/debug/protocol-fee.bin",
        get_deployment_config().with_configurables(protocol_fee_configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "protocolFee: 0x{}",
        ContractId::from(protocol_fee_hook_id.clone())
    );

    protocol_fee_hook_id
}

pub async fn deploy_validator_announce(
    domain: u32,
    mailbox_id: Bech32ContractId,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let validator_announce_configurables = ValidatorAnnounceConfigurables::default()
        .with_LOCAL_DOMAIN(domain)
        .unwrap()
        .with_MAILBOX_ID(mailbox_id.into())
        .unwrap();

    let validator_announce_id = Contract::load_from(
        "../contracts/validator-announce/out/debug/validator-announce.bin",
        get_deployment_config().with_configurables(validator_announce_configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "validatorAnnounce: 0x{}",
        ContractId::from(validator_announce_id.clone())
    );

    validator_announce_id
}

pub async fn deploy_recipient(wallet: &WalletUnlocked) -> Bech32ContractId {
    let recipient_id = Contract::load_from(
        "../contracts/test/msg-recipient-test/out/debug/msg-recipient-test.bin",
        get_deployment_config(),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!("recipient: 0x{}", ContractId::from(recipient_id.clone()));
    recipient_id
}

pub async fn deploy_fallback_domain_routing_hook(
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Bech32ContractId {
    let fallback_domain_routing_configurables = FallbackDomainRoutingHookConfigurables::default()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let fallback_domain_routing_hook_id = Contract::load_from(
        "../contracts/hooks/fallback-domain-routing-hook/out/debug/fallback-domain-routing-hook.bin",
        get_deployment_config().with_configurables(fallback_domain_routing_configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await.unwrap();

    println!(
        "fallbackDomainRoutingHook: 0x{}",
        ContractId::from(fallback_domain_routing_hook_id.clone())
    );

    fallback_domain_routing_hook_id
}

pub async fn deploy_wrapped_asset_minter(
    wallet: &WalletUnlocked,
    asset_registry_id: ContractId,
) -> Bech32ContractId {
    let binary_filepath = "../contracts/wrapped-asset-minter/out/debug/wrapped-asset-minter.bin";
    let minter_id = Contract::load_from(
        binary_filepath,
        get_deployment_config(),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    let minter = WrappedAssetMinter::new(minter_id.clone(), wallet.clone());
    
    // Initialize minter with asset registry
    minter
        .methods()
        .initialize(
            Identity::Address(wallet.address().into()),
            Bits256(*asset_registry_id),
        )
        .call()
        .await
        .unwrap();

    println!(
        "wrappedAssetMinter: 0x{}",
        ContractId::from(minter_id.clone())
    );

    minter_id
}

pub async fn deploy_gas_oracle(wallet_bits: Bits256, wallet: &WalletUnlocked) -> Bech32ContractId {
    let gas_oracle_configurables = GasOracleConfigurables::default()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let gas_oracle_id = Contract::load_from(
        "../contracts/gas-oracle/out/debug/gas-oracle.bin",
        get_deployment_config().with_configurables(gas_oracle_configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "storageGasOracle: 0x{}",
        ContractId::from(gas_oracle_id.clone())
    );

    gas_oracle_id
}

pub async fn deploy_igp(wallet_bits: Bits256, wallet: &WalletUnlocked) -> Bech32ContractId {
    let igp_configurables = GasPaymasterConfigurables::default()
        .with_TOKEN_EXCHANGE_RATE_SCALE(15_000_000_000_000)
        .unwrap()
        .with_DEFAULT_GAS_AMOUNT(5000)
        .unwrap()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let igp_id = Contract::load_from(
        "../contracts/hooks/gas-paymaster/out/debug/gas-paymaster.bin",
        get_deployment_config().with_configurables(igp_configurables),
    )
    .unwrap()
    .deploy(wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "interchainGasPaymaster: 0x{}",
        ContractId::from(igp_id.clone())
    );

    igp_id
}

pub async fn deploy_domain_isms(
    domains: Vec<u32>,
    domain_validators: HashMap<u32, Vec<String>>,
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
) -> Vec<(u32, Bech32ContractId)> {
    println!("Setting up each domain");
    let mut results = Vec::new();
    for domain in domains.into_iter() {
        println!("Setting up domain {}", domain);
        // Get validators for this domain.
        let validators = domain_validators
            .get(&domain)
            .expect("Missing validators for domain");
        let domain_validators_vec = validators
            .iter()
            .map(|validator| EvmAddress::from(Bits256::from_hex_str(validator).unwrap()))
            .collect::<Vec<EvmAddress>>();
        let domain_validator_count = domain_validators_vec.len() as u8;
        println!("Validators for domain {}", domain_validator_count);
        let approval_count = match domain_validator_count {
            1 => 1, // TODO: change later, for testnet should be 1/1
            _ => domain_validator_count - 1,
        };

        // Deploy and instantiate the aggregation ISM.
        let domain_agg_ism_id = deploy_aggregation_ism(wallet_bits, &wallet).await;
        let domain_agg_ism = AggregationISM::new(domain_agg_ism_id.clone(), wallet.clone());

        // Deploy and initialize message ID multisig ISM.
        let message_id_multisig_ism_id =
            deploy_message_id_multisig_ism(wallet_bits, &wallet, approval_count).await;
        let message_id_multisig_ism =
            MessageIdMultisigISM::new(message_id_multisig_ism_id.clone(), wallet.clone());
        let init_res = message_id_multisig_ism
            .methods()
            .initialize(domain_validators_vec.clone())
            .clone()
            .call()
            .await;
        assert!(
            init_res.is_ok(),
            "Failed to initialize message id multisig ISM"
        );
        println!(
            "Initialized message ID multisig ISM with {} validators",
            domain_validator_count
        );

        // Deploy and initialize merkle root multisig ISM.
        let merkle_root_multisig_ism_id =
            deploy_mekle_root_multisig_ism(wallet_bits, &wallet, approval_count).await;
        let merkle_root_multisig_ism =
            MerkleRootMultisigISM::new(merkle_root_multisig_ism_id.clone(), wallet.clone());
        let init_res = merkle_root_multisig_ism
            .methods()
            .initialize(domain_validators_vec.clone())
            .call()
            .await;
        assert!(
            init_res.is_ok(),
            "Failed to initialize merkle root multisig ISM"
        );
        println!(
            "Initialized merkle root multisig ISM with {} validators",
            domain_validator_count
        );

        // Initialize the domain aggregation ISM with the modules.
        let domain_agg_modules = vec![
            message_id_multisig_ism_id.into(),
            merkle_root_multisig_ism_id.into(),
        ];
        let domain_agg_threshold = 1;
        let result = domain_agg_ism
            .methods()
            .initialize(domain_agg_modules, domain_agg_threshold)
            .call()
            .await;
        assert!(
            result.is_ok(),
            "Failed to initialize domain aggregation ISM"
        );
        println!(
            "Initialized domain aggregation ISM 1/2 with message ID and merkle root multisig ISMs"
        );

        // Save the result.
        results.push((domain, domain_agg_ism_id));
    }

    results
}

/// Mainnet setup includes
/// Static Aggregation ISM 2/2
/// Aggregated:
///   - PausableISM
///    - DomainRouting
///    For Every Domain:
///     - Static Aggregation ISM 1/2
///         - MessageIdMultisigISM x/x
///        - MerkleRootMultisigISM x/x
pub async fn deploy_mainnet_ism_setup(
    domains: Vec<u32>,
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
    domain_validators: HashMap<u32, Vec<String>>,
) -> Bech32ContractId {
    // Stage 1
    let pausable_ism_id = deploy_pausable_ism(wallet_bits, wallet).await; // TODO need to initialize owner
    let domain_routing_ism_id = deploy_domain_routing_ism(wallet_bits, wallet).await;

    let top_aggregation_ism_id = deploy_aggregation_ism(wallet_bits, wallet).await;
    let top_aggregation_ism = AggregationISM::new(top_aggregation_ism_id.clone(), wallet.clone());
    let top_level_aggregation_modules =
        vec![pausable_ism_id.into(), domain_routing_ism_id.clone().into()];
    let top_level_aggregation_threshold = top_level_aggregation_modules.len() as u8;
    let result = top_aggregation_ism
        .methods()
        .initialize(
            top_level_aggregation_modules,
            top_level_aggregation_threshold,
        )
        .call()
        .await;
    assert!(result.is_ok(), "Failed to initialize top aggregation ISM");
    println!("Initialized top AGGREGATION ISM 2/2, with PAUSABLE and DOMAIN_ROUTING");

    // Stage 2
    let domain_routing_ism = DomainRoutingISM::new(domain_routing_ism_id, wallet.clone());

    let domains_and_modules =
        deploy_domain_isms(domains, domain_validators, wallet_bits, wallet).await;

    let (domains, modules) = domains_and_modules
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let modules = modules
        .into_iter()
        .map(|module| Bits256(*ContractId::from(module)))
        .collect::<Vec<_>>();

    let wallet_identity = Identity::from(wallet.address());
    let result = domain_routing_ism
        .methods()
        .initialize_with_domains(wallet_identity, domains.clone(), modules)
        .call()
        .await;

    assert!(result.is_ok(), "Failed to initialize domain routing ISM");
    println!(
        "Initialized domain routing ISM with {} domains and modules",
        domains.len()
    );

    top_aggregation_ism_id
}

/// Mainnet setup includes
/// Fallback Domain Routing Hook
/// - Fallback: Merkle Tree Hook
/// For Every Domain:
///  - Merkle Tree Hook
///  - Interchain Gas Paymaster
///  - Pausable Hook
pub async fn deploy_mainnet_hook_setup(
    mailbox_id: Bech32ContractId,
    domains: Vec<u32>,
    wallet_bits: Bits256,
    wallet: &WalletUnlocked,
    domain_gas_configs: Vec<RemoteGasDataConfig>,
) -> Bech32ContractId {
    let wallet_identity = Identity::from(wallet.address());

    let fallback_domain_routing_hook_id =
        deploy_fallback_domain_routing_hook(wallet_bits, wallet).await;
    let merkle_tree_hook_id = deploy_merkle_tree_hook(wallet_bits, wallet).await;
    let merkle_tree_hook = MerkleTreeHook::new(merkle_tree_hook_id.clone(), wallet.clone());
    merkle_tree_hook
        .methods()
        .initialize(mailbox_id)
        .call()
        .await
        .unwrap();

    let fallback_domain_routing_hook =
        FallbackDomainRoutingHook::new(fallback_domain_routing_hook_id.clone(), wallet.clone());

    let res = fallback_domain_routing_hook
        .methods()
        .initialize(
            wallet_identity,
            Bits256(*ContractId::from(merkle_tree_hook_id.clone())),
        )
        .call()
        .await;

    assert!(
        res.is_ok(),
        "Failed to initialize fallback domain routing hook"
    );
    println!("Initialized fallback domain routing hook, with merkle tree hook fallback");

    let pausable_hook_id = deploy_pausable_hook(wallet_bits, wallet).await;
    let gas_oracle_id = deploy_gas_oracle(wallet_bits, wallet).await;
    let gas_paymaster_id = deploy_igp(wallet_bits, wallet).await;

    let gas_oracle = GasOracle::new(gas_oracle_id.clone(), wallet.clone());
    gas_oracle
        .methods()
        .initialize_ownership(wallet_identity)
        .call()
        .await
        .unwrap();
    gas_oracle
        .methods()
        .set_remote_gas_data_configs(domain_gas_configs)
        .call()
        .await
        .unwrap();
    let gas_paymaster = GasPaymaster::new(gas_paymaster_id.clone(), wallet.clone());
    gas_paymaster
        .methods()
        .initialize(wallet_identity, wallet_identity)
        .call()
        .await
        .unwrap();
    // TODO different overheads
    let destination_gas_configs = domains
        .iter()
        .map(|_| DomainGasConfig {
            gas_oracle: Bits256(*ContractId::from(gas_oracle_id.clone())),
            gas_overhead: 151966,
        })
        .collect::<Vec<_>>();
    gas_paymaster
        .methods()
        .set_destination_gas_config(domains.clone(), destination_gas_configs)
        .call()
        .await
        .unwrap();
    println!("Initialized gas oracle and interchain gas paymaster");

    let main_aggregation_hook_id = deploy_aggregation_hook(wallet_bits, wallet).await;
    let main_aggregation_hook =
        AggregationHook::new(main_aggregation_hook_id.clone(), wallet.clone());
    let main_aggregate_hooks = vec![
        pausable_hook_id.into(),
        merkle_tree_hook_id.into(),
        gas_paymaster_id.into(),
    ];
    // TODO loop and make it one aggregation hook per domain
    main_aggregation_hook
        .methods()
        .initialize(main_aggregate_hooks)
        .call()
        .await
        .unwrap();

    let hook_configs = domains
        .iter()
        .map(|domain| HookConfig {
            destination: *domain,
            hook: Bits256(*ContractId::from(main_aggregation_hook_id.clone())),
        })
        .collect::<Vec<_>>();
    fallback_domain_routing_hook
        .methods()
        .set_hooks(hook_configs)
        .call()
        .await
        .unwrap();
    println!("Initialized main aggregation hook");

    fallback_domain_routing_hook_id
}
