use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
};

use fuels::types::{AssetId, ContractId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ContractAddresses {
    #[serde(rename = "mailbox")]
    mailbox: String,
    #[serde(rename = "postDispatch")]
    post_dispatch: String,
    #[serde(rename = "testRecipient")]
    recipient: String,
    #[serde(rename = "interchainSecurityModule")]
    ism: String,
    #[serde(rename = "merkleTreeHook")]
    merkle_tree_hook: String,
    #[serde(rename = "interchainGasPaymaster")]
    igp: String,
    #[serde(rename = "validatorAnnounce")]
    va: String,
    #[serde(rename = "gasOracle")]
    gas_oracle: String,
    #[serde(rename = "aggregationISM")]
    aggregation_ism: String,
    #[serde(rename = "domainRoutingISM")]
    domain_routing_ism: String,
    #[serde(rename = "fallbackDomainRoutingISM")]
    fallback_domain_routing_ism: String,
    #[serde(rename = "messageIdMultisigISM1")]
    message_id_multisig_ism_1: String,
    #[serde(rename = "merkleRootMultisigISM1")]
    merkle_root_multisig_ism_1: String,
    #[serde(rename = "messageIdMultisigISM3")]
    message_id_multisig_ism_3: String,
    #[serde(rename = "merkleRootMultisigISM3")]
    merkle_root_multisig_ism_3: String,
    #[serde(rename = "warpRouteNative")]
    warp_route_native: String,
    #[serde(rename = "warpRouteSynthetic")]
    warp_route_synthetic: String,
    #[serde(rename = "warpRouteCollateral")]
    warp_route_collateral: String,
    #[serde(rename = "collateralTokenContract")]
    collateral_asset_contract_id: String,
    #[serde(rename = "testCollateralAsset")]
    collateral_asset_id: String,
    #[serde(rename = "aggregationHook")]
    aggregation_hook: String,
    #[serde(rename = "pausableHook")]
    pausable_hook: String,
    #[serde(rename = "protocolFee")]
    protocol_fee: String,
    #[serde(rename = "assetRegistry")]
    asset_registry: String,
    #[serde(rename = "wrappedAssetMinter")]
    wrapped_asset_minter: String,
}

#[allow(clippy::too_many_arguments)]
impl ContractAddresses {
    pub fn new(
        mailbox: ContractId,
        post_dispatch: ContractId,
        recipient: ContractId,
        ism: ContractId,
        merkle_tree_hook: ContractId,
        igp: ContractId,
        va: ContractId,
        gas_oracle: ContractId,
        aggregation_ism: ContractId,
        domain_routing_ism: ContractId,
        fallback_domain_routing_ism: ContractId,
        message_id_multisig_ism_1: ContractId,
        merkle_root_multisig_ism_1: ContractId,
        message_id_multisig_ism_3: ContractId,
        merkle_root_multisig_ism_3: ContractId,
        warp_route_native: ContractId,
        warp_route_synthetic: ContractId,
        warp_route_collateral: ContractId,
        collateral_asset_contract_id: ContractId,
        collateral_asset_id: AssetId,
        aggregation_hook: ContractId,
        pausable_hook: ContractId,
        protocol_fee: ContractId,
        asset_registry: ContractId,
        wrapped_asset_minter: ContractId,
    ) -> Self {
        Self {
            mailbox: format!("0x{}", mailbox),
            post_dispatch: format!("0x{}", post_dispatch),
            recipient: format!("0x{}", recipient),
            ism: format!("0x{}", ism),
            merkle_tree_hook: format!("0x{}", merkle_tree_hook),
            igp: format!("0x{}", igp),
            va: format!("0x{}", va),
            gas_oracle: format!("0x{}", gas_oracle),
            aggregation_ism: format!("0x{}", aggregation_ism),
            domain_routing_ism: format!("0x{}", domain_routing_ism),
            fallback_domain_routing_ism: format!("0x{}", fallback_domain_routing_ism),
            message_id_multisig_ism_1: format!("0x{}", message_id_multisig_ism_1),
            merkle_root_multisig_ism_1: format!("0x{}", merkle_root_multisig_ism_1),
            message_id_multisig_ism_3: format!("0x{}", message_id_multisig_ism_3),
            merkle_root_multisig_ism_3: format!("0x{}", merkle_root_multisig_ism_3),
            warp_route_native: format!("0x{}", warp_route_native),
            warp_route_synthetic: format!("0x{}", warp_route_synthetic),
            warp_route_collateral: format!("0x{}", warp_route_collateral),
            collateral_asset_id: format!("0x{}", collateral_asset_id),
            collateral_asset_contract_id: format!("0x{}", collateral_asset_contract_id),
            aggregation_hook: format!("0x{}", aggregation_hook),
            pausable_hook: format!("0x{}", pausable_hook),
            protocol_fee: format!("0x{}", protocol_fee),
            asset_registry: format!("0x{}", asset_registry),
            wrapped_asset_minter: format!("0x{}", wrapped_asset_minter),
        }
    }

    pub fn dump(&self, dump_path: &str) {
        let yaml = serde_yaml::to_string(self).unwrap();
        let full_path = format!("{}/contract_addresses.yaml", dump_path);
        let path = Path::new(&full_path);

        if let Some(parent) = path.parent() {
            create_dir_all(parent).unwrap();
        }
        let mut file = File::create(full_path.clone()).unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        println!("Contract addresses dumped to: {}", full_path);
    }
}
