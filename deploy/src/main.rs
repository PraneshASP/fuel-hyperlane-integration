#![allow(unused)] // TODO remove
use alloy::signers::{
    k256::{ecdsa::SigningKey, SecretKey as SepoliaPrivateKey},
    local::PrivateKeySigner,
};
use core::panic;
use fuels::{
    prelude::*,
    types::{Bits256, ContractId, Salt},
    types::{EvmAddress, Identity},
};
use rand::{thread_rng, Rng};
use std::str::FromStr;
use std::{collections::HashMap, env as std_env};

mod abis;
mod deployers;
mod dump;
mod env;
use abis::*;
use deployers::*;
use dump::*;
use env::*;

async fn update_domain_hooks(fuel_wallet: &WalletUnlocked, wallet_bits: Bits256) {
    // let domain = 84532; base
    // let domain = 11155111; Sepolia
    let domain = 421614; // Arbitrum
    let routing_id =
        ContractId::from_str("0x3e68f0d38374d488e5ae459b28179102bb1e81835c9efbf4cff13e0fce308923")
            .unwrap();
    let domain_routing_hook = FallbackDomainRoutingHook::new(routing_id, fuel_wallet.clone());
    let main_aggregation_hook =
        ContractId::from_str("0xab88a751dbe578311fff4ff559f3953a3cca0805826f7c65cd6b153a31a3bd11")
            .unwrap();

    domain_routing_hook
        .methods()
        .set_hook(domain, Bits256(*main_aggregation_hook))
        .call()
        .await
        .unwrap();

    println!("Added aggregation hook for domain {}", domain);

    let gas_oracle_id =
        ContractId::from_str("0x23e3703017b1b333c3a855ee17ee64093fc9e32458515949778890c0c6bc4c64")
            .unwrap();
    let gas_paymaster_id =
        ContractId::from_str("0xeb7405943f4b0f36d93135b95562f3d510eaefb01b30334d747876da77dfc532")
            .unwrap();

    let gas_config = DomainGasConfig {
        gas_oracle: Bits256(*ContractId::from(gas_oracle_id.clone())),
        gas_overhead: 151966,
    };
    let gas_data = vec![RemoteGasDataConfig {
        domain,
        remote_gas_data: RemoteGasData {
            domain,
            token_exchange_rate: 15000000000,
            gas_price: 16131199970,
            token_decimals: 18,
        },
    }];
    let gas_oracle = GasOracle::new(gas_oracle_id, fuel_wallet.clone());
    let gas_paymaster = GasPaymaster::new(gas_paymaster_id, fuel_wallet.clone());

    // set on oracle
    gas_oracle
        .methods()
        .set_remote_gas_data_configs(gas_data)
        .call()
        .await
        .unwrap();
    println!("Set gas oracle data");
    // set on igp
    gas_paymaster
        .methods()
        .set_destination_gas_config(vec![domain], vec![gas_config])
        .call()
        .await
        .unwrap();
    println!("Set igp data");
}

async fn enroll_ism_domain(fuel_wallet: &WalletUnlocked, wallet_bits: Bits256) {
    let chain_id_to_enroll = 421614;

    let routing_id =
        ContractId::from_str("0x0ead70dc630e8d4fca8a3b697932eac1bca89fac26199df8fbc94fbd7859d340")
            .unwrap();
    let domain_routing_ism = DomainRoutingISM::new(routing_id, fuel_wallet.clone());
    let mut domain_validators = HashMap::new();
    // Chain validators
    domain_validators.insert(
        chain_id_to_enroll,
        vec![zero_pad("0x09fAbFBca0b8Bf042e2A1161Ee5010d147b0f603")],
    );

    let ism_to_add_for_base = deploy_domain_isms(
        vec![chain_id_to_enroll],
        domain_validators.clone(),
        wallet_bits,
        &fuel_wallet,
    )
    .await;

    for (domain, ism) in ism_to_add_for_base {
        println!("Adding domain {} to routing ISM.", domain);
        let init_res = domain_routing_ism
            .methods()
            .set(domain, Bits256(*ContractId::from(ism)))
            .call()
            .await;
        assert!(init_res.is_ok(), "Failed to add domain to routing ISM.");
        println!("Domain {} added to routing ISM.", domain);
    }
}

async fn deploy_mainnet_structure(
    env: DeploymentEnv,
    fuel_wallet: WalletUnlocked,
    wallet_bits: Bits256,
) {
    let remote_domains = get_remote_domain_ids();
    // TODO change to env variable
    // Make DOMAIn_THRESHOLDS `{domain_id:threshold, domain_id:threshold,...}`
    // let threshold = 1;
    let mut domain_validators = HashMap::new();
    let mut domain_gas_data = vec![];
    for domain in remote_domains.clone() {
        domain_validators.insert(
            domain,
            vec![
                zero_pad("0x469F0940684D147Defc44F3647146CB90Dd0BC8E"),
                zero_pad("0xb22B65F202558ADF86A8BB2847B76AE1036686a5"),
                zero_pad("0xd3C75Dcf15056012a4d74C483A0C6ea11d8c2b83"),
            ],
        );
        domain_gas_data.push(RemoteGasDataConfig {
            domain,
            remote_gas_data: RemoteGasData {
                domain,
                token_exchange_rate: 15000000000,
                gas_price: 16131199970,
                token_decimals: 18,
            },
        });
    }

    let mailbox_contract_id = deploy_mailbox(env.origin_domain, wallet_bits, &fuel_wallet).await;

    let default_ism = deploy_mainnet_ism_setup(
        remote_domains.clone(),
        wallet_bits,
        &fuel_wallet,
        domain_validators,
    )
    .await;
    let default_hook = deploy_mainnet_hook_setup(
        mailbox_contract_id.clone(),
        remote_domains,
        wallet_bits,
        &fuel_wallet,
        domain_gas_data,
    )
    .await;
    let required_hook = deploy_protocol_fee_hook(wallet_bits, &fuel_wallet).await;

    // Deploy and initialize wrapped asset minter and asset registry
    let minter_id = deploy_wrapped_asset_minter(&fuel_wallet, ContractId::zeroed()).await;
    let asset_registry_id = deploy_asset_registry(&fuel_wallet, ContractId::from(minter_id.clone())).await;
    
    // Update minter with actual asset registry address
    let minter = WrappedAssetMinter::new(minter_id.clone(), fuel_wallet.clone());
    minter
        .methods()
        .initialize(
            Identity::Address(fuel_wallet.address().into()),
            Bits256(*ContractId::from(asset_registry_id.clone())),
        )
        .call()
        .await
        .unwrap();

    let recipient_id = deploy_recipient(&fuel_wallet).await;
    let validator_announce_id =
        deploy_validator_announce(env.origin_domain, mailbox_contract_id.clone(), &fuel_wallet)
            .await;

    let wallet_identity = Identity::from(fuel_wallet.address());
    let mailbox = Mailbox::new(mailbox_contract_id.clone(), fuel_wallet.clone());
    mailbox
        .methods()
        .initialize(
            wallet_identity,
            Bits256(*ContractId::from(default_ism)),
            Bits256(*ContractId::from(default_hook)),
            Bits256(*ContractId::from(required_hook)),
        )
        .call()
        .await
        .unwrap();

    println!("Mailbox initialized.");

    println!("Deployment complete.");
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    // Wallet Initialization
    let env = DeploymentEnv::new();
    let fuel_provider = Provider::connect(env.rpc_url).await.unwrap();
    let fuel_wallet =
        WalletUnlocked::new_from_private_key(env.secret_key, Some(fuel_provider.clone()));
    let block_number = fuel_provider.latest_block_height().await.unwrap();
    let wallet_bits = Bits256(fuel_wallet.address().hash().into());
    let config = get_deployment_config();
    println!("Deployer: {}", Address::from(fuel_wallet.address()));
    println!("Config sync block: {}", block_number);

    if env.structure == "hyperlane" {
        return deploy_mainnet_structure(env, fuel_wallet, wallet_bits).await;
    }
    if env.structure == "enroll-ism" {
        return enroll_ism_domain(&fuel_wallet, wallet_bits).await;
    }
    if env.structure == "update-hooks" {
        return update_domain_hooks(&fuel_wallet, wallet_bits).await;
    }

    /////////////////////////////////
    // Mailbox Contract Deployment //
    /////////////////////////////////

    let mailbox_contract_id = deploy_mailbox(env.origin_domain, wallet_bits, &fuel_wallet).await;

    ///////////////////////////////////
    // Post Dispatch Mock Deployment //
    ///////////////////////////////////

    let binary_filepath = "../contracts/mocks/mock-post-dispatch/out/debug/mock-post-dispatch.bin";
    let contract = Contract::load_from(binary_filepath, config.clone()).unwrap();
    let post_dispatch_mock_id = contract
        .deploy(&fuel_wallet, TxPolicies::default())
        .await
        .unwrap();

    println!(
        "postDispatch: 0x{}",
        ContractId::from(post_dispatch_mock_id.clone())
    );

    ///////////////////////////////
    // Test Recipient deployment //
    ///////////////////////////////

    let recipient_id = deploy_recipient(&fuel_wallet).await;

    /////////////////////
    // ISMs deployment //
    /////////////////////

    let test_ism_id = Contract::load_from(
        "../contracts/test/ism-test/out/debug/ism-test.bin",
        config.clone(),
    )
    .unwrap()
    .deploy(&fuel_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "interchainSecurityModule: 0x{}",
        ContractId::from(test_ism_id.clone())
    );

    let aggregation_ism_id = deploy_aggregation_ism(wallet_bits, &fuel_wallet).await;

    let domain_routing_ism_id = deploy_domain_routing_ism(wallet_bits, &fuel_wallet).await;

    let configurables = FallbackDomainRoutingISMConfigurables::default()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();

    let fallback_domain_routing_ism_id = Contract::load_from(
        "../contracts/ism/routing/default-fallback-domain-routing-ism/out/debug/default-fallback-domain-routing-ism.bin",
        config.clone().with_configurables(configurables),
    )
    .unwrap()
    .deploy(&fuel_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "FallbackDomainRoutingISM: 0x{}",
        ContractId::from(fallback_domain_routing_ism_id.clone())
    );

    let message_id_multisig_ism_id_1 =
        deploy_message_id_multisig_ism(wallet_bits, &fuel_wallet, 1).await;

    let message_id_multisig_ism_id_3 =
        deploy_message_id_multisig_ism(wallet_bits, &fuel_wallet, 3).await;

    let merkle_root_multisig_ism_id_1 =
        deploy_mekle_root_multisig_ism(wallet_bits, &fuel_wallet, 1).await;

    let merkle_root_multisig_ism_id_3 =
        deploy_mekle_root_multisig_ism(wallet_bits, &fuel_wallet, 3).await;

    /////////////////////////////////
    // Merkle Tree hook deployment //
    /////////////////////////////////

    let merkle_tree_id = deploy_merkle_tree_hook(wallet_bits, &fuel_wallet).await;

    /////////////////////////////////
    // Aggregation Hook Deployment //
    /////////////////////////////////

    let aggregation_hook_id = deploy_aggregation_hook(wallet_bits, &fuel_wallet).await;

    ///////////////////////////////
    // Pausable Hook Deployment //
    //////////////////////////////

    let pausable_hook_id = deploy_pausable_hook(wallet_bits, &fuel_wallet).await;

    //////////////////////////////////
    // Protocol Fee Hook Deployment //
    //////////////////////////////////

    let protocol_fee_hook_id = deploy_protocol_fee_hook(wallet_bits, &fuel_wallet).await;

    ///////////////////////////////////
    // Asset Registry and Minter Deployment //
    ///////////////////////////////////

    let minter_id = deploy_wrapped_asset_minter(&fuel_wallet, ContractId::zeroed()).await;
    let asset_registry_id = deploy_asset_registry(&fuel_wallet, ContractId::from(minter_id.clone())).await;
    
    // Update minter with actual asset registry address
    let minter = WrappedAssetMinter::new(minter_id.clone(), fuel_wallet.clone());
    // minter
    //     .methods()
    //     .initialize(
    //         Identity::Address(fuel_wallet.address().into()),
    //         Bits256(*ContractId::from(asset_registry_id.clone())),
    //     )
    //     .call()
    //     .await
    //     .unwrap();

    /////////////////////////////////////////
    // Gas Paymaster Components Deployment //
    /////////////////////////////////////////

    let gas_oracle_id = deploy_gas_oracle(wallet_bits, &fuel_wallet).await;

    let igp_id = deploy_igp(wallet_bits, &fuel_wallet).await;

    ///////////////////////////
    // Warp Route Deployment //
    ///////////////////////////

    //Collateral Token
    let collateral_token_salt = Salt::from(rand::thread_rng().gen::<[u8; 32]>());
    let collateral_asset_contract_id = Contract::load_from(
        "../contracts/test/src20-test/out/debug/src20-test.bin",
        config.clone().with_salt(collateral_token_salt),
    )
    .unwrap()
    .deploy(&fuel_wallet, TxPolicies::default())
    .await
    .unwrap();

    let collateral_token_contract =
        SRC20Test::new(collateral_asset_contract_id.clone(), fuel_wallet.clone());

    let _ = collateral_token_contract
        .methods()
        .mint(
            Identity::Address(fuel_wallet.address().into()),
            Some(Bits256::zeroed()),
            2 * 10_u64.pow(18),
        )
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        .call()
        .await
        .unwrap();

    println!(
        "collateralTokenContractId: {}",
        collateral_token_contract.contract_id()
    );

    let collateral_asset_id = collateral_asset_contract_id.asset_id(&Bits256::zeroed());
    println!("collateralAssetId: 0x{}", collateral_asset_id.clone());

    //Collateral WR
    let wr_configurables = WarpRouteConfigurables::default()
        .with_EXPECTED_OWNER(wallet_bits)
        .unwrap();
    let collateral_salt = Salt::from(rand::thread_rng().gen::<[u8; 32]>());
    let warp_route_collateral_id = Contract::load_from(
        "../contracts/warp-route/out/debug/warp-route.bin",
        config
            .clone()
            .with_salt(collateral_salt)
            .with_configurables(wr_configurables.clone()),
    )
    .unwrap()
    .deploy(&fuel_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "warpRouteCollateral: 0x{}",
        ContractId::from(warp_route_collateral_id.clone())
    );

    // Native WR
    let native_salt = Salt::from(rand::thread_rng().gen::<[u8; 32]>());
    let warp_route_native_id = Contract::load_from(
        "../contracts/warp-route/out/debug/warp-route.bin",
        config
            .clone()
            .with_salt(native_salt)
            .with_configurables(wr_configurables.clone()),
    )
    .unwrap()
    .deploy(&fuel_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "warpRouteNative: 0x{}",
        ContractId::from(warp_route_native_id.clone())
    );

    // Synthetic WR
    let synthetic_salt = Salt::from(rand::thread_rng().gen::<[u8; 32]>());
    let warp_route_synthetic_id = Contract::load_from(
        "../contracts/warp-route/out/debug/warp-route.bin",
        config
            .clone()
            .with_salt(synthetic_salt)
            .with_configurables(wr_configurables),
    )
    .unwrap()
    .deploy(&fuel_wallet, TxPolicies::default())
    .await
    .unwrap();

    println!(
        "warpRouteSynthetic: 0x{}",
        ContractId::from(warp_route_synthetic_id.clone())
    );

    ///////////////////////////
    // Instantiate Contracts //
    ///////////////////////////

    let post_dispatch_mock = PostDispatch::new(post_dispatch_mock_id.clone(), fuel_wallet.clone());
    let mailbox = Mailbox::new(mailbox_contract_id.clone(), fuel_wallet.clone());
    let merkle_tree_hook = MerkleTreeHook::new(merkle_tree_id.clone(), fuel_wallet.clone());
    let aggregation_hook = AggregationHook::new(aggregation_hook_id.clone(), fuel_wallet.clone());
    let pausable_hook = PausableHook::new(pausable_hook_id.clone(), fuel_wallet.clone());
    let protocol_fee_hook = ProtocolFee::new(protocol_fee_hook_id.clone(), fuel_wallet.clone());
    let gas_oracle = GasOracle::new(gas_oracle_id.clone(), fuel_wallet.clone());
    let igp = GasPaymaster::new(igp_id.clone(), fuel_wallet.clone());
    let test_recipient = TestRecipient::new(recipient_id.clone(), fuel_wallet.clone());
    let aggregation_ism = AggregationISM::new(aggregation_ism_id.clone(), fuel_wallet.clone());
    let domain_routing_ism =
        DomainRoutingISM::new(domain_routing_ism_id.clone(), fuel_wallet.clone());
    let fallback_domain_routing_ism =
        FallbackDomainRoutingISM::new(fallback_domain_routing_ism_id.clone(), fuel_wallet.clone());
    let message_id_multisig_ism_1 =
        MessageIdMultisigISM::new(message_id_multisig_ism_id_1.clone(), fuel_wallet.clone());
    let merkle_root_multisig_ism_1 =
        MerkleRootMultisigISM::new(merkle_root_multisig_ism_id_1.clone(), fuel_wallet.clone());
    let message_id_multisig_ism_3 =
        MessageIdMultisigISM::new(message_id_multisig_ism_id_3.clone(), fuel_wallet.clone());
    let merkle_root_multisig_ism_3 =
        MerkleRootMultisigISM::new(merkle_root_multisig_ism_id_3.clone(), fuel_wallet.clone());
    let warp_route_native = WarpRoute::new(warp_route_native_id.clone(), fuel_wallet.clone());
    let warp_route_synthetic = WarpRoute::new(warp_route_synthetic_id.clone(), fuel_wallet.clone());
    let warp_route_collateral =
        WarpRoute::new(warp_route_collateral_id.clone(), fuel_wallet.clone());

    let wallet_identity = Identity::from(fuel_wallet.address());
    let test_ism_address = Bits256(ContractId::from(test_ism_id.clone()).into());
    let mailbox_address = Bits256(ContractId::from(mailbox_contract_id.clone()).into());

    /////////////////////
    // Initialize ISMs //
    /////////////////////

    // Aggregation ISM
    let aggregation_ism_threshold = 2;
    let test_isms_to_aggregate = vec![
        ContractId::from(test_ism_id.clone()),
        ContractId::from(test_ism_id.clone()),
    ];
    let init_res = aggregation_ism
        .methods()
        .initialize(test_isms_to_aggregate, aggregation_ism_threshold)
        .call()
        .await;

    assert!(init_res.is_ok(), "Failed to initialize Aggregation ISM.");

    // Domain Routing ISM
    let init_res = domain_routing_ism
        .methods()
        .initialize_with_domains(
            wallet_identity,
            vec![11155111, 84532],
            vec![test_ism_address, test_ism_address],
        )
        .call()
        .await;

    assert!(init_res.is_ok(), "Failed to initialize Domain Routing ISM.");

    // Fallback Domain Routing ISM
    let init_res = fallback_domain_routing_ism
        .methods()
        .initialize(wallet_identity, mailbox_address)
        .call()
        .await;

    assert!(
        init_res.is_ok(),
        "Failed to initialize Fallback Domain Routing ISM."
    );

    // Multisig ISMs validator setup
    // (Threshold is set during contract deployment)

    let evm_pk_vars = [
        "SEPOLIA_PRIVATE_KEY_1",
        "SEPOLIA_PRIVATE_KEY_2",
        "SEPOLIA_PRIVATE_KEY_3",
    ];

    let validators_to_enroll = evm_pk_vars
        .iter()
        .map(|pk| {
            let secret_key = SepoliaPrivateKey::from_slice(
                &hex::decode(std_env::var(pk).unwrap_or_else(|_| panic!("{:?} must be set", pk)))
                    .unwrap(),
            )
            .unwrap();
            let signing_key = SigningKey::from(secret_key);
            let signer = PrivateKeySigner::from_signing_key(signing_key);
            EvmAddress::from(Bits256(signer.address().into_word().0))
        })
        .collect::<Vec<_>>();

    // Message ID Multisig ISM, threshold 1
    let init_res = message_id_multisig_ism_1
        .methods()
        .initialize(validators_to_enroll.clone())
        .call()
        .await;
    assert!(
        init_res.is_ok(),
        "Failed to initailize Message ID Multisig ISM, threshold 1."
    );

    // Message ID Multisig ISM, threshold 3
    let init_res = message_id_multisig_ism_3
        .methods()
        .initialize(validators_to_enroll.clone())
        .call()
        .await;
    assert!(
        init_res.is_ok(),
        "Failed to initailize Message ID Multisig ISM, threshold 3."
    );

    // Merkle Root Multisig ISM, threshold 1
    let init_res = merkle_root_multisig_ism_1
        .methods()
        .initialize(validators_to_enroll.clone())
        .call()
        .await;
    assert!(
        init_res.is_ok(),
        "Failed to initailize Merkle Root Multisig ISM, threshold 1."
    );

    // Merkle Root Multisig ISM, threshold 3
    let init_res = merkle_root_multisig_ism_3
        .methods()
        .initialize(validators_to_enroll.clone())
        .call()
        .await;
    assert!(
        init_res.is_ok(),
        "Failed to initailize Merkle Root Multisig ISM, threshold 3."
    );

    /////////////////////////
    // Test Recipiet Setup //
    /////////////////////////

    let set_res = test_recipient
        .methods()
        .set_ism(test_ism_id.clone())
        .call()
        .await;

    assert!(set_res.is_ok(), "Failed to set ISM in Test Recipient.");

    ////////////////////////////////
    // Initalize Mailbox Contract //
    ////////////////////////////////

    let post_dispatch_mock_address = Bits256(ContractId::from(post_dispatch_mock.id()).into());

    let init_res = mailbox
        .methods()
        .initialize(
            wallet_identity,
            test_ism_address,
            post_dispatch_mock_address, // Initially set to mocks
            post_dispatch_mock_address,
        )
        .call()
        .await;
    assert!(init_res.is_ok(), "Failed to initialize Mailbox.");
    println!("Mailbox initialized.");

    ///////////////////////////////
    // Initialize IGP Components //
    ///////////////////////////////

    let owner_identity = Identity::Address(Address::from(fuel_wallet.address()));

    // Initialize contracts
    let init_res = gas_oracle
        .methods()
        .initialize_ownership(owner_identity)
        .call()
        .await;
    assert!(init_res.is_ok(), "Failed to initialize Gas Oracle.");

    let init_res = igp
        .methods()
        .initialize(wallet_identity, wallet_identity)
        .call()
        .await;
    assert!(init_res.is_ok(), "Failed to initialize IGP.");

    // Gas Oracle
    let set_gas_data_res = gas_oracle
        .methods()
        .set_remote_gas_data_configs(vec![
            RemoteGasDataConfig {
                domain: 84532, // For Testnet Demo
                remote_gas_data: RemoteGasData {
                    domain: 84532,
                    // Numbers from BSC and Optimism testnets - 15000000000
                    token_exchange_rate: 15000000000,
                    gas_price: 37999464941,
                    token_decimals: 18,
                },
            },
            RemoteGasDataConfig {
                domain: 9913371, // For local E2E test
                remote_gas_data: RemoteGasData {
                    domain: 9913371,
                    // Numbers from BSC and Optimism testnets - 15000000000
                    token_exchange_rate: 15000000000,
                    gas_price: 37999464941,
                    token_decimals: 18,
                },
            },
        ])
        .call()
        .await;
    assert!(set_gas_data_res.is_ok(), "Failed to set gas data.");

    // IGP
    let set_beneficiary_res = igp.methods().set_beneficiary(owner_identity).call().await;
    assert!(set_beneficiary_res.is_ok(), "Failed to set beneficiary.");

    let set_gas_oracle_res = igp
        .methods()
        .set_gas_oracle(84532, Bits256(gas_oracle_id.hash().into()))
        .call()
        .await;

    assert!(set_gas_data_res.is_ok(), "Failed to set gas data.");
    assert!(set_beneficiary_res.is_ok(), "Failed to set beneficiary.");
    assert!(set_gas_oracle_res.is_ok(), "Failed to set gas oracle.");

    ////////////////////////
    // Validator Announce //
    ////////////////////////

    let validator_announce_id =
        deploy_validator_announce(env.origin_domain, mailbox_contract_id.clone(), &fuel_wallet)
            .await;

    /////////////////////////////////////
    // Asset Registry Initialization //
    /////////////////////////////////////

    let minter_id = deploy_wrapped_asset_minter(&fuel_wallet, ContractId::zeroed()).await;
    let asset_registry_id = deploy_asset_registry(&fuel_wallet, ContractId::from(minter_id.clone())).await;
    
    // Update minter with actual asset registry address
    let minter = WrappedAssetMinter::new(minter_id.clone(), fuel_wallet.clone());
    // minter
    //     .methods()
    //     .initialize(
    //         Identity::Address(fuel_wallet.address().into()),
    //         Bits256(*ContractId::from(asset_registry_id.clone())),
    //     )
    //     .call()
    //     .await
    //     .unwrap();

    /////////////////////////////////////
    // Merkle Tree Hook Initialization //
    /////////////////////////////////////

    let init_res = merkle_tree_hook
        .methods()
        .initialize(mailbox.id())
        .call()
        .await;
    assert!(init_res.is_ok(), "Failed to initialize Merkle Tree Hook.");
    println!("Merkle Tree Hook initialized.");

    ///////////////////////////////
    // Pausable Hook Initialization //
    ///////////////////////////////
    let init_res = pausable_hook
        .methods()
        .initialize_ownership(owner_identity)
        .call()
        .await;
    assert!(init_res.is_ok(), "Failed to initialize Pausable Hook.");
    println!("Pausable Hook initialized.");

    //////////////////////////////////////
    // Protocol Fee Hook Initialization //
    //////////////////////////////////////

    let protocol_fee = 1;

    let init_res = protocol_fee_hook
        .methods()
        .initialize(protocol_fee, owner_identity, owner_identity)
        .call()
        .await;
    assert!(init_res.is_ok(), "Failed to initialize Protocol Fee Hook.");
    println!("Protocol Fee Hook initialized.");

    //////////////////////////////////////
    // Aggregation Hook Initialization //
    //////////////////////////////////////

    let hooks = vec![post_dispatch_mock_id.clone().into(), igp_id.clone().into()];

    let init_res = aggregation_hook.methods().initialize(hooks).call().await;
    assert!(init_res.is_ok(), "Failed to initialize Aggregation Hook.");
    println!("Aggregation Hook initialized.");

    ///////////////////////////////
    // Warp Route Initialization //
    ///////////////////////////////

    // Initalize Warp Routes
    let native_init_res = warp_route_native
        .methods()
        .initialize(
            wallet_identity,
            Bits256(mailbox_contract_id.hash().into()),
            WarpRouteTokenMode::NATIVE,
            post_dispatch_mock_address,
            test_ism_address,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .call()
        .await;

    assert!(
        native_init_res.is_ok(),
        "Failed to initialize Warp Route Native."
    );

    let synthetic_init_res = warp_route_synthetic
        .methods()
        .initialize(
            wallet_identity,
            Bits256(mailbox_contract_id.hash().into()),
            WarpRouteTokenMode::SYNTHETIC,
            post_dispatch_mock_address,
            test_ism_address,
            Some("FuelSepoliaUSDC".to_string()),
            Some("FST".to_string()),
            Some(6),
            Some(10_000_000),
            None,
            None,
        )
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        .call()
        .await;

    assert!(
        synthetic_init_res.is_ok(),
        "Failed to initialize Warp Route Synthetic."
    );

    let collateral_init_res = warp_route_collateral
        .methods()
        .initialize(
            wallet_identity,
            Bits256(mailbox_contract_id.hash().into()),
            WarpRouteTokenMode::COLLATERAL,
            post_dispatch_mock_address,
            test_ism_address,
            None,
            None,
            None,
            None,
            Some(collateral_asset_id),
            Some(Bits256(collateral_asset_contract_id.hash().into())),
        )
        .with_contract_ids(&[collateral_asset_contract_id.clone()])
        .call()
        .await;

    assert!(
        collateral_init_res.is_ok(),
        "Failed to initialize Warp Route Collateral."
    );

    /////////////////////////////
    // Save contract addresses //
    /////////////////////////////

    ContractAddresses::new(
        mailbox_contract_id.into(),
        post_dispatch_mock_id.into(),
        recipient_id.into(),
        test_ism_id.into(),
        merkle_tree_id.into(),
        igp_id.into(),
        validator_announce_id.into(),
        gas_oracle_id.into(),
        aggregation_ism_id.into(),
        domain_routing_ism_id.into(),
        fallback_domain_routing_ism_id.into(),
        message_id_multisig_ism_id_1.into(),
        merkle_root_multisig_ism_id_1.into(),
        message_id_multisig_ism_id_3.into(),
        merkle_root_multisig_ism_id_3.into(),
        warp_route_native_id.into(),
        warp_route_synthetic_id.into(),
        warp_route_collateral_id.into(),
        collateral_asset_contract_id.into(),
        collateral_asset_id,
        aggregation_hook_id.into(),
        pausable_hook_id.into(),
        protocol_fee_hook_id.into(),
        asset_registry_id.into(),
        minter_id.into()
    )
    .dump(&env.dump_path);
}

fn get_deployment_config() -> LoadConfiguration {
    let mut rng = thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes[..]);
    bytes.reverse();
    let salt = Salt::new(bytes);

    LoadConfiguration::default().with_salt(salt)
}
