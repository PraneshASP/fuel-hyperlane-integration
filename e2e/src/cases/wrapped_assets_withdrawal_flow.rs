use crate::{
    cases::TestCase,
    setup::{
        abis::{AssetRegistry, Mailbox, PostDispatchHook, ProtocolFee, WrappedAssetMinter},
        get_loaded_wallet,
    },
    utils::{
        create_mock_metadata, get_evm_domain,
        local_contracts::get_contract_address_from_yaml,
        token::{
            get_balance, get_contract_balance, get_local_fuel_base_asset, get_native_asset,
            send_gas_to_contract, send_gas_to_contract_2,
        },
    },
};
use fuels::{
    programs::calls::CallParameters,
    types::{transaction_builders::VariableOutputPolicy, Bits256, Bytes, Identity, U256},
};
use tokio::time::Instant;

pub fn test() -> TestCase {
    TestCase::new(
        "wrapped_assets_withdrawal_flow",
        wrapped_assets_withdrawal_flow,
    )
}

async fn wrapped_assets_withdrawal_flow() -> std::result::Result<f64, String> {
    let start = Instant::now();
    println!("Starting wrapped assets withdrawal flow test...");

    let wallet = get_loaded_wallet().await;

    let mailbox_id = get_contract_address_from_yaml("mailbox");
    let hook_id = get_contract_address_from_yaml("postDispatch");
    let minter_id = get_contract_address_from_yaml("wrappedAssetMinter");
    let registry_id = get_contract_address_from_yaml("assetRegistry");
    // let igp_id = get_contract_address_from_yaml("interchainGasPaymaster");
    // let gas_oracle_id = get_contract_address_from_yaml("gasOracle");
    // let ism_id = get_contract_address_from_yaml("interchainSecurityModule");
    let protocol_fee_hook_id = get_contract_address_from_yaml("protocolFee");

    let minter = WrappedAssetMinter::new(minter_id.clone(), wallet.clone());
    let registry = AssetRegistry::new(registry_id.clone(), wallet.clone());
    let mailbox = Mailbox::new(mailbox_id.clone(), wallet.clone());
    let hook = PostDispatchHook::new(hook_id.clone(), wallet.clone());
    let protocol_fee_hook = ProtocolFee::new(protocol_fee_hook_id, wallet.clone());

    let origin_chain_id = get_evm_domain();
    let token_address_hex = "0x0000000000000000000000009E545E3C0baAB3E08CdfD552C960A1050f373042";
    let token_address = Bits256::from_hex_str(token_address_hex).unwrap();

    registry
        .methods()
        .update_mailbox(mailbox_id.clone())
        .call()
        .await
        .unwrap();

    registry
        .methods()
        .set_hook(hook_id.clone())
        .call()
        .await
        .unwrap();

    println!("Retrieving asset details...");
    let sub_id = registry
        .methods()
        .get_sub_id(origin_chain_id as u32, token_address)
        .call()
        .await
        .map_err(|e| format!("Failed to get sub_id: {:?}", e))?
        .value;

    let redemption_ticket_id = registry
        .methods()
        .get_redemption_ticket_for_sub_id(sub_id)
        .call()
        .await
        .map_err(|e| format!("Failed to get redemption ticket ID: {:?}", e))?
        .value
        .unwrap();

    println!("Asset sub_id: {:?}", sub_id);
    println!("Redemption ticket id: {:?}", redemption_ticket_id);

    let wrapped_asset_id = minter.contract_id().asset_id(&sub_id);
    let redemption_ticket_asset_id = minter.contract_id().asset_id(&redemption_ticket_id);
    println!("Wrapped Asset Id: {:?}", wrapped_asset_id);

    let initial_wrapped_balance = get_balance(
        wallet.provider().unwrap(),
        wallet.address(),
        wrapped_asset_id,
    )
    .await
    .unwrap();

    let initial_redemption_balance = get_balance(
        wallet.provider().unwrap(),
        wallet.address(),
        redemption_ticket_asset_id,
    )
    .await
    .unwrap();

    println!("Initial wrapped asset balance: {}", initial_wrapped_balance);
    println!(
        "Initial redemption ticket balance: {}",
        initial_redemption_balance
    );

    let deposit_amount = std::cmp::min(initial_redemption_balance, 500_000u64);
    println!("Depositing {} redemption tickets...", deposit_amount);

    let deposit_result = registry
        .methods()
        .deposit_redemption_tickets(sub_id)
        .with_contracts(&[&minter])
        .call_params(CallParameters::new(
            deposit_amount,
            redemption_ticket_asset_id,
            5_000_000,
        ))
        .unwrap()
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        .call()
        .await
        .map_err(|e| format!("Failed to deposit redemption tickets: {:?}", e))?;

    let internal_redemption_balance = registry
        .methods()
        .get_redemption_balance(Identity::Address(wallet.address().into()), sub_id)
        .call()
        .await
        .map_err(|e| format!("Failed to get redemption balance: {:?}", e))?
        .value;

    println!(
        "Internal redemption balance: {}",
        internal_redemption_balance
    );
    assert!(internal_redemption_balance >= deposit_amount);

    let withdraw_amount = std::cmp::min(deposit_amount / 2, 250_000u64);

    let destination_address = token_address.clone(); //Bits256([1u8; 32]);
    let destination_domain = origin_chain_id as u32;

    // println!(
    //     "Withdrawing {} tokens to external chain...",
    //     withdraw_amount
    // );

    let quote = mailbox
        .methods()
        .quote_dispatch(
            destination_domain,
            destination_address,
            build_message_body(destination_address, withdraw_amount),
            Bytes(vec![]),
            hook_id,
        )
        .with_contracts(&[&hook, &protocol_fee_hook])
        .call()
        .await
        .unwrap()
        .value;

    send_gas_to_contract_2(
        wallet.clone(),
        &registry_id.into(),
        quote,
        get_local_fuel_base_asset(),
    )
    .await;

    let balance = get_contract_balance(
        wallet.provider().unwrap(),
        &registry_id.into(),
        get_local_fuel_base_asset(),
    )
    .await;
    let withdraw_result = registry
        .methods()
        .withdraw_to_external_chain(
            sub_id,
            destination_domain,
            destination_address,
            None,
            Some(hook_id),
        )
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        .call_params(CallParameters::new(
            withdraw_amount,
            wrapped_asset_id,
            10_000_000,
        ))
        // .with_contract_ids(&[
        //     mailbox_id.into(),
        //     igp_id.into(),
        //     gas_oracle_id.into(),
        //     hook_id.into(),
        //   //  ism_id.into(),
        //     minter_id.into()
        // ])
        .unwrap()
        .with_contracts(&[&minter, &mailbox, &hook, &protocol_fee_hook])
        // .determine_missing_contracts(Some(5))
        // .await
        // .unwrap()
        .call()
        .await
        .map_err(|e| format!("Withdraw transaction failed: {:?}", e))?;

    let message_id = withdraw_result.value;
    println!(
        "Withdraw transaction successful! Message ID: {:?}",
        message_id
    );

    assert!(
        message_id != Bits256::zeroed(),
        "Message ID should not be zero"
    );

    let post_withdraw_wrapped_balance = get_balance(
        wallet.provider().unwrap(),
        wallet.address(),
        wrapped_asset_id,
    )
    .await
    .unwrap();

    let post_withdraw_redemption_balance = registry
        .methods()
        .get_redemption_balance(Identity::Address(wallet.address().into()), sub_id)
        .call()
        .await
        .map_err(|e| format!("Failed to get post-withdraw redemption balance: {:?}", e))?
        .value;

    println!(
        "Post-withdraw wrapped balance: {}",
        post_withdraw_wrapped_balance
    );
    println!(
        "Post-withdraw internal redemption balance: {}",
        post_withdraw_redemption_balance
    );

    assert_eq!(
        post_withdraw_wrapped_balance,
        initial_wrapped_balance - withdraw_amount,
    );

    assert_eq!(
        post_withdraw_redemption_balance,
        internal_redemption_balance - withdraw_amount
    );

    println!("✅ wrapped_assets_withdrawal_flow test completed successfully");
    Ok(start.elapsed().as_secs_f64())
}

fn build_message_body(recipient: Bits256, amount: u64) -> Bytes {
    let mut buffer = Vec::new();

    let amount_u256 = U256::from(amount);
    let mut amount_bytes = [0u8; 32];
    amount_u256.to_big_endian(&mut amount_bytes);

    buffer.extend(&recipient.0);
    buffer.extend(&amount_bytes);

    Bytes(buffer)
}
