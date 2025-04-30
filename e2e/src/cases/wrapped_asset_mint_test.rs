use std::str::FromStr;
use tokio::time::Instant;

use fuels::{
    prelude::*,
    types::{Bits256, Bytes, ContractId, Identity},
};

use crate::{
    cases::TestCase,
    setup::{
        abis::{AssetRegistry, Mailbox, PostDispatchHook, WrappedAssetMinter, AggregationISM},
        get_loaded_wallet,
    },
    utils::{get_evm_domain, local_contracts::get_contract_address_from_yaml},
};

use hyperlane_core::{Encode, HyperlaneMessage, H256};

pub fn test() -> TestCase {
    TestCase::new("wrapped_asset_mint_fuel", wrapped_asset_mint)
}

async fn wrapped_asset_mint() -> std::result::Result<f64, String> {
    let start = Instant::now();
    let wallet = get_loaded_wallet().await;

    // fetch IDs
    let mailbox_id = get_contract_address_from_yaml("mailbox");
    let hook_id = get_contract_address_from_yaml("postDispatch");
    let minter_id = get_contract_address_from_yaml("wrappedAssetMinter");
    let registry_id = get_contract_address_from_yaml("assetRegistry");
    let ism_id = get_contract_address_from_yaml("interchainSecurityModule");

    // init instances
    let minter = WrappedAssetMinter::new(minter_id.clone(), wallet.clone());
    let registry = AssetRegistry::new(registry_id.clone(), wallet.clone());
    let mailbox = Mailbox::new(mailbox_id.clone(), wallet.clone());
    let hook = PostDispatchHook::new(hook_id.clone(), wallet.clone());
    let ism = AggregationISM::new(ism_id.clone(), wallet.clone());

    let default_ism = mailbox.methods().default_ism().simulate(Execution::StateReadOnly).await.unwrap().value;
    // println!("Default ISM: {} {}", default_ism, ism_id);
    // println!("Hook: {}", hook_id);
    // println!("minter_id: {}", minter_id);
    // println!("registry_id: {}", registry_id);

    let minter_registry = minter.methods().registry().simulate(Execution::StateReadOnly).await.unwrap().value;
    let _ = minter.methods().set_registry(Bits256(registry_id.into())).call().await;
    // println!("registry_id: {:?}", minter_registry);


    // initialize minter ➔ registry
    // minter
    //     .methods()
    //     .initialize(
    //         Identity::Address(wallet.address().into()),
    //         Bits256(*ContractId::from(registry_id.clone())),
    //     )
    //     .call()
    //     .await
    //     .map_err(|e| e.to_string())?;

    // // initialize registry ➔ minter
    // registry
    //     .methods()
    //     .initialize(
    //         Identity::Address(wallet.address().into()),
    //         ContractId::from(minter_id.clone()),
    //     )
    //     .call()
    //     .await
    //     .map_err(|e| e.to_string())?;

    // mailbox init
    // mailbox
    //     .methods()
    //     .initialize(
    //         Identity::Address(wallet.address().into()),
    //         Bits256(*ContractId::from(hook_id.clone())),
    //         Bits256(*ContractId::from(hook_id.clone())),
    //         Bits256(*ContractId::from(hook_id.clone())),
    //     )
    //     .call()
    //     .await
    //     .map_err(|e| e.to_string())?;

    // register bridge
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

     let origin_chain_id = get_evm_domain(); 
    let raw = hex::decode("1c7d4b196cb0c7b01d743fbc6116a902379c7238").map_err(|e| e.to_string())?;
    let mut token_bytes = [0u8; 32];
    token_bytes[12..].copy_from_slice(&raw);
    let token_address = Bits256(token_bytes);

    let asset_sub_id = registry
        .methods()
        .register_asset(
            origin_chain_id as u64,
            token_address,
            6,
            "USD Coin".into(),
            "USDC".into(),
        )
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;

     registry
        .methods()
        .authorize_bridge_for_asset(bridge_id, asset_sub_id)
        .call()
        .await
        .map_err(|e| e.to_string())?;

    let redemption_ticket_id = registry
        .methods()
        .get_redemption_ticket_for_sub_id(asset_sub_id)
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value;

    let recipient = Bits256(wallet.address().hash().into());
    let mut body = recipient.0.to_vec();
    let mint_amount = 1_000_000_000u64;
    let amt_be = mint_amount.to_be_bytes();
    let mut pad = vec![0u8; 32 - amt_be.len()];
    pad.extend_from_slice(&amt_be);
    body.extend_from_slice(&pad);

    let message: HyperlaneMessage = HyperlaneMessage {
        version: 3,
        nonce: 0,
        origin: origin_chain_id as u32,
        sender: H256::from_slice(&token_address.0),
        destination: 13373,  
        recipient: H256::from_slice(registry.contract_id().hash().as_slice()),
        body,
    };

    let metadata = Bytes(vec![0u8; 32]);
    let message_bytes = Bytes(message.to_vec());
    mailbox
        .methods()
        .process(metadata, message_bytes)
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
         .with_contracts(&[&registry, &hook, &minter, &ism])
        //.with_contract_ids(&[registry.clone().contract_id().clone(), hook.clone().contract_id().clone(), minter.clone().contract_id().clone()])
        .call()
        .await
        .map_err(|e| e.to_string())?;

    let asset_id = minter.contract_id().asset_id(&asset_sub_id);
    let wrapped_balance = wallet
        .get_asset_balance(&asset_id)
        .await
        .map_err(|e| e.to_string())?;
    if wrapped_balance != mint_amount {
        return Err(format!("wrapped {} != {}", wrapped_balance, mint_amount));
    }

    let ticket_asset = minter
        .contract_id()
        .asset_id(&redemption_ticket_id.unwrap());
    let ticket_balance = wallet
        .get_asset_balance(&ticket_asset)
        .await
        .map_err(|e| e.to_string())?;
    if ticket_balance != mint_amount {
        return Err(format!("ticket {} != {}", ticket_balance, mint_amount));
    }

    let total = minter
        .methods()
        .total_supply(asset_id)
        .call()
        .await
        .map_err(|e| e.to_string())?
        .value
        .unwrap_or_default();
    if total != mint_amount {
        return Err(format!("total {} != {}", total, mint_amount));
    }
    println!("✅ wrapped_asset_mint_fuel passed");
    Ok(start.elapsed().as_secs_f64())
}
