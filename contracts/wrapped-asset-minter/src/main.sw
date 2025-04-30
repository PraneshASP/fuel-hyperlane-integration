contract;

use sway_libs::{
    asset::{
        base::{
            _decimals,
            _name,
            _set_decimals,
            _set_name,
            _set_symbol,
            _symbol,
            _total_assets,
            _total_supply,
        },
        supply::{
            _burn,
            _mint,
        },
    },
    ownership::*,
    pausable::*,
    reentrancy::reentrancy_guard,
};

use std::{
    asset::transfer,
    call_frames::msg_asset_id,
    context::{
        balance_of,
        msg_amount,
        this_balance,
    },
    contract_id::ContractId,
    hash::*,
    revert::revert,
    storage::storage_map::*,
    storage::storage_string::*,
    storage::storage_vec::*,
    string::String,
};

use standards::{src20::SRC20, src5::State};
use utils::{
    IssuanceParams,
    redemption_ticket_sub_id,
    TokenDetails,
    WrappedAssetsError,
};

enum WrappedAssetMinterError {
    NotAuthorizedMinter: (),
    AssetNotFound: (),
    InvalidAmount: (),
    InvalidAddress: (),
    RegistryError: (),
}

storage {
    total_assets: u64 = 0,
    total_supply: StorageMap<AssetId, u64> = StorageMap {},
    name: StorageMap<AssetId, StorageString> = StorageMap {},
    symbol: StorageMap<AssetId, StorageString> = StorageMap {},
    decimals: StorageMap<AssetId, u8> = StorageMap {},
    origin_chain: StorageMap<SubId, u64> = StorageMap {},
    origin_address: StorageMap<SubId, b256> = StorageMap {},
    origin_decimals: StorageMap<SubId, u8> = StorageMap {},
    owner: State = State::Uninitialized,
    asset_to_sub_id: StorageMap<AssetId, SubId> = StorageMap {},
    registry: Option<b256> = Option::None,
}

struct MintEvent {
    recipient: Identity,
    asset_id: AssetId,
    sub_id: SubId,
    amount: u64,
}

struct BurnEvent {
    sender: Identity,
    asset_id: AssetId,
    sub_id: SubId,
    amount: u64,
}

struct RedemptionTicketMintEvent {
    recipient: Identity,
    sub_id: SubId,
    redemption_ticket_id: SubId,
    amount: u64,
}

// Metadata type for SRC-7 responses
enum Metadata {
    String: String,
    Number: u64,
    Address: b256,
}

abi AssetRegistry {
    #[storage(read)]
    fn get_token_details(sub_id: b256) -> Option<TokenDetails>;

    #[storage(read)]
    fn get_asset_params(sub_id: b256) -> Option<IssuanceParams>;

    #[storage(read)]
    fn is_bridge_authorized_for_asset(bridge_id: b256, sub_id: b256) -> bool;
}

abi WrappedAssetMinter {

    // SRC-20 Implementation
    // #[storage(read)]
    // fn total_assets() -> u64;

    // #[storage(read)]
    // fn total_supply(asset: AssetId) -> Option<u64>;

    // #[storage(read)]
    // fn name(asset: AssetId) -> Option<String>;

    // #[storage(read)]
    // fn symbol(asset: AssetId) -> Option<String>;

    // #[storage(read)]
    // fn decimals(asset: AssetId) -> Option<u8>;

    // Initialization
    #[storage(read, write)]
    fn initialize(owner: Identity, registry: b256);

    // Minting and burning functions
    #[storage(read, write)]
    fn mint(
        bridge_id: b256,
        recipient: Identity,
        sub_id: SubId,
        amount: u64,
    );

    #[payable]
    #[storage(read, write)]
    fn burn(sub_id: SubId, amount: u64);

    // Metadata functions
    #[storage(read)]
    fn metadata(asset: AssetId, key: String) -> Option<Metadata>;

    // Ownership functions
    #[storage(read)]
    fn owner() -> State;

    #[storage(read, write)]
    fn transfer_ownership(new_owner: Identity);

    #[storage(read, write)]
    fn set_registry(registry: b256);

    #[storage(read)]
    fn registry() -> b256;

    #[storage(read, write)]
    fn mint_redemption_tickets(recipient: Identity, sub_id: SubId, amount: u64);
}

// Implementation of the SRC-20 interface
impl SRC20 for Contract {
    #[storage(read)]
    fn total_assets() -> u64 {
        storage.total_assets.read()
    }

    #[storage(read)]
    fn total_supply(asset: AssetId) -> Option<u64> {
        storage.total_supply.get(asset).try_read()
    }

    #[storage(read)]
    fn name(asset: AssetId) -> Option<String> {
        // Get SubId for the asset
        let sub_id = match storage.asset_to_sub_id.get(asset).try_read() {
            Some(id) => id,
            None => return Option::None,
        };

        // Get token details from registry
        let registry_id = match storage.registry.read() {
            Some(id) => id,
            None => return Option::None,
        };

        let registry = abi(AssetRegistry, registry_id);
        match registry.get_token_details(sub_id) {
            Some(details) => Option::Some(details.name),
            None => Option::None,
        }
    }

    #[storage(read)]
    fn symbol(asset: AssetId) -> Option<String> {
        // Get SubId for the asset
        let sub_id = match storage.asset_to_sub_id.get(asset).try_read() {
            Some(id) => id,
            None => return Option::None,
        };

        // Get token details from registry
        let registry_id = match storage.registry.read() {
            Some(id) => id,
            None => return Option::None,
        };

        let registry = abi(AssetRegistry, registry_id);
        match registry.get_token_details(sub_id) {
            Some(details) => Option::Some(details.symbol),
            None => Option::None,
        }
    }

    #[storage(read)]
    fn decimals(asset: AssetId) -> Option<u8> {
        // Get SubId for the asset
        let sub_id = match storage.asset_to_sub_id.get(asset).try_read() {
            Some(id) => id,
            None => return Option::None,
        };

        // Get token details from registry
        let registry_id = match storage.registry.read() {
            Some(id) => id,
            None => return Option::None,
        };

        let registry = abi(AssetRegistry, registry_id);
        match registry.get_token_details(sub_id) {
            Some(details) => Option::Some(details.decimals),
            None => Option::None,
        }
    }
}

impl WrappedAssetMinter for Contract {
    #[storage(read, write)]
    fn initialize(owner: Identity, registry: b256) {
        require(
            storage
                .owner
                .read() == State::Uninitialized,
            "Already initialized",
        );

        storage.owner.write(State::Initialized(owner));
        storage.registry.write(Some(registry));
    }

    #[storage(read, write)]
    fn set_registry(registry: b256) {
        storage.registry.write(Some(registry));
    }

     #[storage(read)]
    fn registry() -> b256 {
        let registry_id = match storage.registry.read() {
            Some(id) => id,
            None => {
                b256::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            },
        };
        return registry_id;
    }

    #[storage(read, write)]
    fn mint(
        bridge_id: b256,
        recipient: Identity,
        sub_id: SubId,
        amount: u64,
    ) {
        reentrancy_guard();

        // TODO: add a check to validate if msg_sender == registry
        require(amount > 0, WrappedAssetMinterError::InvalidAmount);

        // Check if bridge is authorized via registry
        let registry_id = match storage.registry.read() {
            Some(id) => id,
            None => {
                require(false, "Registry not set");
                b256::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            },
        };

        let registry = abi(AssetRegistry, registry_id);
        require(
            registry
                .is_bridge_authorized_for_asset(bridge_id, sub_id),
            WrappedAssetMinterError::NotAuthorizedMinter,
        );

        // Get asset details from registry
        let params = match registry.get_asset_params(sub_id) {
            Some(p) => p,
            None => {
                require(false, "Asset not found");
                IssuanceParams {
                    asset_id: AssetId::from(0x0000000000000000000000000000000000000000000000000000000000000000),
                    sub_id: 0x0000000000000000000000000000000000000000000000000000000000000000,
                    origin_chain_id: 0,
                    origin_token_address: 0x0000000000000000000000000000000000000000000000000000000000000000,
                    origin_decimals: 0,
                }
            },
        };

        // Store asset to sub_id mapping if not exists
        let asset_id = AssetId::new(ContractId::this(), sub_id);
        if storage.asset_to_sub_id.get(asset_id).try_read().is_none()
        {
            storage.asset_to_sub_id.insert(asset_id, sub_id);
        }

        let current_supply = storage.total_supply.get(asset_id).try_read().unwrap_or(0);

        _mint(
            storage
                .total_assets,
            storage
                .total_supply,
            recipient,
            sub_id,
            amount,
        );
    }

    #[storage(read, write)]
    fn mint_redemption_tickets(recipient: Identity, sub_id: SubId, amount: u64) {
        let registry = storage.registry.read().unwrap();
        // require(
        //     msg_sender().unwrap() == Identity::ContractId(ContractId::from(registry)),
        //     "Caller not registry"
        // );

        let redemption_ticket_id = redemption_ticket_sub_id(sub_id);

        _mint(
            storage
                .total_assets,
            storage
                .total_supply,
            recipient,
            redemption_ticket_id,
            amount,
        );

        let asset_id = AssetId::new(ContractId::this(), redemption_ticket_id);
        if storage.asset_to_sub_id.get(asset_id).try_read().is_none()
        {
            storage
                .asset_to_sub_id
                .insert(asset_id, redemption_ticket_id);
        }

        log(RedemptionTicketMintEvent {
            recipient,
            sub_id,
            redemption_ticket_id,
            amount,
        });
    }

    #[payable]
    #[storage(read, write)]
    fn burn(sub_id: SubId, amount: u64) {
        reentrancy_guard();

        require(
            msg_amount() >= amount,
            WrappedAssetMinterError::InvalidAmount,
        );

        let asset_id = AssetId::new(ContractId::this(), sub_id);
        require(
            msg_asset_id() == asset_id,
            WrappedAssetMinterError::InvalidAddress,
        );

        // Update supply
        let current_supply = storage.total_supply.get(asset_id).try_read().unwrap_or(0);
        require(
            current_supply >= amount,
            WrappedAssetMinterError::InvalidAmount,
        );
        // Burn the tokens
        _burn(storage.total_supply, sub_id, amount);

        // log(BurnEvent {
        //     sender: msg_sender().unwrap(),
        //     asset_id,
        //     sub_id,
        //     amount,
        // });
    }

    #[storage(read)]
    fn metadata(asset: AssetId, key: String) -> Option<Metadata> {
        let sub_id = match storage.asset_to_sub_id.get(asset).try_read() {
            Some(id) => id,
            None => return Option::None,
        };

        // Get registry
        let registry_id = match storage.registry.read() {
            Some(id) => id,
            None => return Option::None,
        };

        let registry = abi(AssetRegistry, registry_id);

        let params = match registry.get_asset_params(sub_id) {
            Some(p) => p,
            None => return Option::None,
        };

        let bridged_chain = String::from_ascii_str("bridged:chain");
        if key == bridged_chain {
            return Some(Metadata::Number(params.origin_chain_id));
        }

        let bridged_address = String::from_ascii_str("bridged:address");
        if key == bridged_address {
            return Some(Metadata::Address(params.origin_token_address));
        }

        let bridged_decimals = String::from_ascii_str("bridged:decimals");
        if key == bridged_decimals {
            return Some(Metadata::Number(params.origin_decimals.into()));
        }

        Option::None
    }

    #[storage(read)]
    fn owner() -> State {
        storage.owner.read()
    }

    #[storage(read, write)]
    fn transfer_ownership(new_owner: Identity) {
        let current_state = storage.owner.read();
        require(
            current_state == State::Initialized(msg_sender().unwrap()),
            WrappedAssetMinterError::NotAuthorizedMinter,
        );

        storage.owner.write(State::Initialized(new_owner));
    }
}
