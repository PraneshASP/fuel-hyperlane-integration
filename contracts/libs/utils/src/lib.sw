library;
use std::{
    auth::msg_sender,
    hash::{Hash, sha256},
    string::String,
    storage::storage_map::StorageMap,
    storage::storage_string::*,
    option::Option,
};

pub struct IssuanceParams {
    /// The native asset ID on Fuel
    pub asset_id: AssetId,
    /// The sub ID used to derive the asset ID
    pub sub_id: b256,
    /// The chain ID of the origin chain (e.g., 1 for Ethereum)
    pub origin_chain_id: u64,
    /// The token address on the origin chain
    pub origin_token_address: b256,
    /// The number of decimal places for the token on the origin chain
    pub origin_decimals: u8,
}

/// Computes the sub ID for a given origin chain and token address
pub fn compute_sub_id(origin_chain_id: u64, origin_token_address: b256) -> b256 {
    sha256((origin_chain_id, origin_token_address))
}

pub enum WrappedAssetsError {
    /// The sender is not authorized to perform this action
    Unauthorized: (),
    /// The bridge is not registered
    BridgeNotRegistered: (),
    /// The asset is not registered
    AssetNotRegistered: (),
    /// The asset is already registered
    AssetAlreadyRegistered: (),
    /// Invalid parameters were provided
    InvalidParameters: (),
    /// The bridge is not authorized for this asset
    BridgeNotAuthorizedForAsset: (),
}

/// Checks if the sender is authorized to perform an action
pub fn require_authorized(authorized_identity: Option<Identity>) -> Result<Identity, WrappedAssetsError> {
    let sender = msg_sender().unwrap();
    match authorized_identity {
        Some(identity) => {
            if identity == sender {
                Ok(sender)
            } else {
                Err(WrappedAssetsError::Unauthorized)
            }
        },
        None => Err(WrappedAssetsError::Unauthorized),
    }
}

pub struct TokenDetails {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

pub fn redemption_ticket_sub_id(asset_sub_id: b256) -> b256 {
    let REDEMPTION_TICKET_PREFIX = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
    asset_sub_id ^ REDEMPTION_TICKET_PREFIX
}