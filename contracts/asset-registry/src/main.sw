contract;

use std::{
    auth::msg_sender,
    block::timestamp,
    bytes::Bytes,
    call_frames::msg_asset_id,
    constants::ZERO_B256,
    context::msg_amount,
    hash::{
        Hash,
        sha256,
    },
    identity::Identity,
    option::Option,
    storage::storage_map::*,
    storage::storage_string::*,
    storage::storage_vec::*,
    string::String,
};

use utils::{
    compute_sub_id,
    IssuanceParams,
    redemption_ticket_sub_id,
    require_authorized,
    TokenDetails,
    WrappedAssetsError,
};

storage {
    owner: Option<Identity> = Option::None,
    authorized_bridges_per_asset: StorageMap<b256, StorageMap<b256, bool>> = StorageMap {},
    minter_contract_id: Option<ContractId> = Option::None,
    asset_parameters: StorageMap<b256, IssuanceParams> = StorageMap {},
    token_names: StorageMap<b256, StorageString> = StorageMap {},
    token_symbols: StorageMap<b256, StorageString> = StorageMap {},
    registered_bridges: StorageMap<b256, bool> = StorageMap {},
    // bridge_address -> bridge_id
    bridge_ids: StorageMap<Identity, b256> = StorageMap {},
    redemption_ticket_to_subid: StorageMap<b256, b256> = StorageMap {},
    subid_to_redemption_ticket: StorageMap<b256, b256> = StorageMap {},
    redemption_balances: StorageMap<(Identity, b256), u64> = StorageMap {},
    /// Each domain has a unique router that handles token transfers
    routers: StorageMap<u32, b256> = StorageMap {},
    ///List of unique domains
    domains: StorageVec<u32> = StorageVec {},
    /// Mapping of remote router decimals
    remote_router_decimals: StorageMap<b256, u8> = StorageMap {},
    mailbox_contract_id: Option<ContractId> = Option::None,
    default_hook: ContractId = ContractId::zero(),
}

enum RegistryEvent {
    BridgeRegistered: (String, b256),
    AssetRegistered: (b256, IssuanceParams),
    BridgeAuthorizedForAsset: (b256, b256),
    BridgeDeauthorizedForAsset: (b256, b256),
    OwnerUpdated: Identity,
    MinterContractUpdated: ContractId,
    MailboxContractUpdated: ContractId,
}

enum TokenRouterError {
    RouterNotSet: (),
    RouterLengthMismatch: (),
}

abi TokenRouter {
    #[storage(read)]
    fn router(domain: u32) -> b256;

    #[storage(read)]
    fn all_routers() -> Vec<b256>;

    #[storage(read)]
    fn all_domains() -> Vec<u32>;

    /// Removes a router for a specific domain
    #[storage(read, write)]
    fn unenroll_remote_router(domain: u32) -> bool;

    #[storage(read, write)]
    fn enroll_remote_router(domain: u32, router: b256);

    #[storage(read, write)]
    fn enroll_remote_routers(domains: Vec<u32>, routers: Vec<b256>);

    #[storage(read)]
    fn remote_router_decimals(router: b256) -> u8;

    #[storage(read, write)]
    fn set_remote_router_decimals(router: b256, decimals: u8);
}

abi MessageRecipient {
    #[storage(read)]
    fn handle(origin: u32, sender: b256, message_body: Bytes);

    #[storage(read)]
    fn extract_asset_data_from_body(message_body: Bytes);

    #[storage(read)]
    fn interchain_security_module() -> ContractId;
}

abi WrappedAssetMinter {
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

    #[storage(read, write)]
    fn mint_redemption_tickets(recipient: Identity, sub_id: SubId, amount: u64);
}

abi UniversalWrappedAssetsRegistry {
    #[storage(read, write)]
    fn initialize(owner: Identity, minter_contract: ContractId);

    #[storage(read, write)]
    fn update_owner(new_owner: Identity);

    #[storage(read, write)]
    fn update_minter_contract(new_minter_contract: ContractId);

    #[storage(read, write)]
    fn update_mailbox(new_mailbox: ContractId);

    // bridge_contract should be the address that calls the `handle()` method
    // For ex: hyperlane mailbox
    #[storage(read, write)]
    fn register_bridge(bridge_name: String, bridge_contract: Identity) -> b256;

    #[storage(read, write)]
    fn register_asset(
        origin_chain_id: u64,
        origin_token_address: b256,
        origin_decimals: u8,
        name: String,
        symbol: String,
    ) -> b256;

    #[storage(read)]
    fn get_asset_params(sub_id: b256) -> Option<IssuanceParams>;

    #[storage(read)]
    fn get_token_details(sub_id: b256) -> Option<TokenDetails>;

    #[storage(read)]
    fn is_asset_registered(sub_id: b256) -> bool;

    #[storage(read)]
    fn is_bridge_registered(bridge_id: b256) -> bool;

    #[storage(read)]
    fn is_bridge_authorized_for_asset(bridge_id: b256, sub_id: b256) -> bool;

    #[storage(read, write)]
    fn authorize_bridge_for_asset(bridge_id: b256, sub_id: b256);

    #[storage(read, write)]
    fn deauthorize_bridge_for_asset(bridge_id: b256, sub_id: b256);

    #[storage(read)]
    fn get_sub_id(origin: u32, token_address: b256) -> b256;

    #[storage(read)]
    fn get_bridge_id_storage() -> b256;

    #[storage(read)]
    fn get_minter() -> ContractId;

    #[storage(read)]
    fn get_mailbox() -> ContractId;

    #[storage(read)]
    fn minter_mint(
        origin: u32,
        sender: b256,
        recipient: b256,
        amount: u64,
        minter_id: ContractId,
        bridge_id: b256,
    );

    #[storage(read)]
    fn is_redemption_ticket(sub_id: b256) -> bool;

    #[storage(read)]
    fn get_sub_id_for_redemption_ticket(redemption_ticket_id: b256) -> Option<b256>;

    #[storage(read)]
    fn get_redemption_ticket_for_sub_id(sub_id: b256) -> Option<b256>;

    #[storage(read, write), payable]
    fn deposit_redemption_tickets(sub_id: b256) -> u64;

    #[storage(read, write), payable]
    fn withdraw_to_external_chain(
        sub_id: b256,
        destination_domain: u32,
        recipient: b256,
        metadata: Option<Bytes>,
        hook: Option<ContractId>,
    ) -> b256;

    #[storage(read)]
    fn get_redemption_balance(user: Identity, asset_sub_id: b256) -> u64;

    #[storage(read)]
    fn get_hook() -> ContractId;

    #[storage(write)]
    fn set_hook(hook: ContractId);
}

abi Mailbox {
    #[storage(read)]
    fn dispatch(
        destination_domain: u32,
        recipient: b256,
        message_body: Bytes,
        metadata: Bytes,
        hook: ContractId,
    ) -> b256;
}

impl UniversalWrappedAssetsRegistry for Contract {
    #[storage(read, write)]
    fn initialize(owner: Identity, minter_contract: ContractId) {
        let current_owner = storage.owner.read();
        require(current_owner.is_none(), "Contract already initialized");

        storage.owner.write(Option::Some(owner));
        storage
            .minter_contract_id
            .write(Option::Some(minter_contract));

        log(RegistryEvent::OwnerUpdated(owner));
        log(RegistryEvent::MinterContractUpdated(minter_contract));
    }

    #[storage(read, write)]
    fn update_owner(new_owner: Identity) {
        let _ = require_authorized(storage.owner.read());

        storage.owner.write(Option::Some(new_owner));

        log(RegistryEvent::OwnerUpdated(new_owner));
    }

    #[storage(read, write)]
    fn update_minter_contract(new_minter_contract: ContractId) {
        let _ = require_authorized(storage.owner.read());

        storage
            .minter_contract_id
            .write(Option::Some(new_minter_contract));

        log(RegistryEvent::MinterContractUpdated(new_minter_contract));
    }

    #[storage(read, write)]
    fn update_mailbox(new_mailbox: ContractId) {
        let _ = require_authorized(storage.owner.read());

        storage.mailbox_contract_id.write(Option::Some(new_mailbox));

        log(RegistryEvent::MailboxContractUpdated(new_mailbox));
    }

    #[storage(read, write)]
    fn register_bridge(bridge_name: String, bridge_address: Identity) -> b256 {
        let _ = require_authorized(storage.owner.read());

        let bridge_id = sha256(bridge_name);
        storage.registered_bridges.insert(bridge_id, true);
        storage.bridge_ids.insert(bridge_address, bridge_id);

        log(RegistryEvent::BridgeRegistered((bridge_name, bridge_id)));

        bridge_id
    }

    // Maintains a whitelist of assets (admin-only)
    #[storage(read, write)]
    fn register_asset(
        origin_chain_id: u64,
        origin_token_address: b256,
        origin_decimals: u8,
        name: String,
        symbol: String,
    ) -> b256 {
        let _ = require_authorized(storage.owner.read());

        let minter_contract = storage.minter_contract_id.read();
        require(minter_contract.is_some(), "Minter contract not set");

        let sub_id = compute_sub_id(origin_chain_id, origin_token_address);

        let existing_params = storage.asset_parameters.get(sub_id).try_read();
        require(existing_params.is_none(), "Asset already registered");

        let asset_id = AssetId::new(minter_contract.unwrap(), sub_id);
        let params = IssuanceParams {
            asset_id,
            sub_id,
            origin_chain_id,
            origin_token_address,
            origin_decimals,
        };

        storage.asset_parameters.insert(sub_id, params);

        storage.token_names.insert(sub_id, StorageString {});
        storage.token_symbols.insert(sub_id, StorageString {});

        storage.token_names.get(sub_id).write_slice(name);
        storage.token_symbols.get(sub_id).write_slice(symbol);

        let redemption_ticket_id = redemption_ticket_sub_id(sub_id);

        storage
            .redemption_ticket_to_subid
            .insert(redemption_ticket_id, sub_id);
        storage
            .subid_to_redemption_ticket
            .insert(sub_id, redemption_ticket_id);

        let rt_name = String::from_ascii_str("Redemption Ticket");
        let rt_symbol = String::from_ascii_str("RT");

        storage
            .token_names
            .insert(redemption_ticket_id, StorageString {});
        storage
            .token_symbols
            .insert(redemption_ticket_id, StorageString {});

        // TODO: concat it with the asset name and symbol
        storage
            .token_names
            .get(redemption_ticket_id)
            .write_slice(rt_name);
        storage
            .token_symbols
            .get(redemption_ticket_id)
            .write_slice(rt_symbol);

        log(RegistryEvent::AssetRegistered((sub_id, params)));

        sub_id
    }

    #[storage(read)]
    fn get_asset_params(sub_id: b256) -> Option<IssuanceParams> {
        storage.asset_parameters.get(sub_id).try_read()
    }

    #[storage(read)]
    fn get_token_details(sub_id: b256) -> Option<TokenDetails> {
        let name = storage.token_names.get(sub_id).read_slice();
        let symbol = storage.token_symbols.get(sub_id).read_slice();

        if (storage.redemption_ticket_to_subid.get(sub_id).try_read().is_some())
        {
            Option::Some(TokenDetails {
                name: name.unwrap(),
                symbol: symbol.unwrap(),
                decimals: 9u8,
            })
        } else {
            let params = storage.asset_parameters.get(sub_id).try_read();

            if name.is_none() || symbol.is_none() || params.is_none() {
                return Option::None;
            }

            Option::Some(TokenDetails {
                name: name.unwrap(),
                symbol: symbol.unwrap(),
                decimals: params.unwrap().origin_decimals,
            })
        }
    }

    #[storage(read)]
    fn is_asset_registered(sub_id: b256) -> bool {
        storage.asset_parameters.get(sub_id).try_read().is_some()
    }

    #[storage(read)]
    fn is_bridge_registered(bridge_id: b256) -> bool {
        storage.registered_bridges.get(bridge_id).try_read().unwrap_or(false)
    }

    #[storage(read)]
    fn is_bridge_authorized_for_asset(bridge_id: b256, sub_id: b256) -> bool {
        if !storage.registered_bridges.get(bridge_id).try_read().unwrap_or(false)
        {
            return false;
        }

        if storage.asset_parameters.get(sub_id).try_read().is_none()
        {
            return false;
        }
        storage.authorized_bridges_per_asset.get(sub_id).get(bridge_id).try_read().unwrap_or(false)
    }

    #[storage(read, write)]
    fn authorize_bridge_for_asset(bridge_id: b256, sub_id: b256) {
        let _ = require_authorized(storage.owner.read());

        require(
            storage
                .registered_bridges
                .get(bridge_id)
                .try_read()
                .unwrap_or(false),
            "Bridge not registered",
        );

        require(
            storage
                .asset_parameters
                .get(sub_id)
                .try_read()
                .is_some(),
            "Asset not registered",
        );

        if storage.authorized_bridges_per_asset.get(sub_id).try_read().is_none()
        {
            storage
                .authorized_bridges_per_asset
                .insert(sub_id, StorageMap {});
        }

        storage
            .authorized_bridges_per_asset
            .get(sub_id)
            .insert(bridge_id, true);

        log(RegistryEvent::BridgeAuthorizedForAsset((bridge_id, sub_id)));
    }

    #[storage(read, write)]
    fn deauthorize_bridge_for_asset(bridge_id: b256, sub_id: b256) {
        let _ = require_authorized(storage.owner.read());
        log(storage.authorized_bridges_per_asset.get(sub_id).try_read().is_none());
        storage
            .authorized_bridges_per_asset
            .get(sub_id)
            .insert(bridge_id, false);
        log(RegistryEvent::BridgeDeauthorizedForAsset((bridge_id, sub_id)));
    }

    #[storage(read)]
    fn get_sub_id(origin: u32, token_address: b256) -> b256 {
        return compute_sub_id(origin.into(), token_address);
    }

    #[storage(read)]
    fn get_bridge_id_storage() -> b256 {
        storage.bridge_ids.get(msg_sender().unwrap()).read()
    }

    #[storage(read)]
    fn get_minter() -> ContractId {
        let minter_id = match storage.minter_contract_id.read() {
            Some(id) => id,
            None => {
                ContractId::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            },
        };

        minter_id
    }

    #[storage(read)]
    fn get_mailbox() -> ContractId {
        let mailbox_id = match storage.mailbox_contract_id.read() {
            Some(id) => id,
            None => {
                ContractId::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            },
        };

        mailbox_id
    }

    #[storage(read)]
    fn minter_mint(
        origin: u32,
        sender: b256,
        recipient: b256,
        amount: u64,
        minter_id: ContractId,
        bridge_id: b256,
    ) {
        let recipient_identity = Identity::Address(Address::from(recipient));

        let token_address = sender;

        let sub_id = compute_sub_id(origin.into(), token_address);
        let minter = abi(WrappedAssetMinter, b256::from(minter_id));

        minter.mint(bridge_id, recipient_identity, sub_id, amount);

        minter.mint_redemption_tickets(Identity::Address(Address::from(recipient)), sub_id, amount);
    }

    #[storage(read, write), payable]
    fn deposit_redemption_tickets(sub_id: b256) -> u64 {
        let redemption_ticket_id = match storage.subid_to_redemption_ticket.get(sub_id).try_read() {
            Some(id) => id,
            None => {
                require(false, "Asset not registered");
                b256::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            }
        };

        let minter_id = storage.minter_contract_id.read().unwrap();
        let redemption_asset_id = AssetId::new(minter_id, redemption_ticket_id);

        require(msg_asset_id() == redemption_asset_id, "Wrong asset sent");
        let amount = msg_amount();
        require(amount > 0, "Amount must be greater than zero");

        let sender = msg_sender().unwrap();
        let balance_key = (sender, sub_id);
        let current_balance = storage.redemption_balances.get(balance_key).try_read().unwrap_or(0);
        let new_balance = current_balance + amount;
        storage.redemption_balances.insert(balance_key, new_balance);

        new_balance
    }

    #[payable]
    #[storage(read, write)]
    fn withdraw_to_external_chain(
        sub_id: b256,
        destination_domain: u32,
        recipient: b256,
        metadata: Option<Bytes>,
        hook: Option<ContractId>,
    ) -> b256 {
        let sender = msg_sender().unwrap();
        let balance_key = (sender, sub_id);

        let redemption_balance = storage.redemption_balances.get(balance_key).try_read().unwrap_or(0);
        let amount = msg_amount();
        require(
            redemption_balance >= amount,
            "Insufficient redemption balance",
        );

        let minter_id = storage.minter_contract_id.read().unwrap();
        let asset_id = AssetId::new(minter_id, sub_id);
        require(msg_asset_id() == asset_id, "Wrong asset sent");

        // TODO: Fix auth  
        // let bridge_id = storage.bridge_ids.get(sender).read();
        // require(storage
        //         .registered_bridges
        //         .get(bridge_id)
        //         .try_read()
        //         .unwrap_or(false), "Only bridges can withdraw");

        storage
            .redemption_balances
            .insert(balance_key, redemption_balance - amount);

        let minter = abi(WrappedAssetMinter, b256::from(minter_id));
        minter
            .burn {
                coins: amount,
                asset_id: asset_id.into(),
            }(sub_id, amount);

        let redemption_ticket_id = match storage.subid_to_redemption_ticket.get(sub_id).try_read() {
            Some(id) => id,
            None => {
                require(false, "Asset not registered");
                b256::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            }
        };
        let redemption_asset_id = AssetId::new(minter_id, redemption_ticket_id);
        minter
            .burn {
                coins: amount,
                asset_id: redemption_asset_id.into(),
            }(redemption_ticket_id, amount);

        // TODO: Build message body for the bridge
        let message_body = _build_message_body(recipient, amount);

        let mailbox_id = match storage.mailbox_contract_id.read() {
            Some(id) => id,
            None => {
                require(false, "Mailbox not set");
                ContractId::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            },
        };
        let mailbox = abi(Mailbox, b256::from(mailbox_id));
        let message_id = mailbox.dispatch(
            destination_domain,
            recipient,
            message_body,
            metadata
                .unwrap_or(Bytes::from(b256::zero())),
            hook
                .unwrap_or(storage.default_hook.read()),
        );

        message_id
        //         ZERO_B256
    }

    #[storage(read)]
    fn is_redemption_ticket(sub_id: b256) -> bool {
        storage.redemption_ticket_to_subid.get(sub_id).try_read().is_some()
    }

    #[storage(read)]
    fn get_sub_id_for_redemption_ticket(redemption_ticket_id: b256) -> Option<b256> {
        storage.redemption_ticket_to_subid.get(redemption_ticket_id).try_read()
    }

    #[storage(read)]
    fn get_redemption_ticket_for_sub_id(sub_id: b256) -> Option<b256> {
        storage.subid_to_redemption_ticket.get(sub_id).try_read()
    }

    #[storage(read)]
    fn get_redemption_balance(user: Identity, asset_sub_id: b256) -> u64 {
        storage.redemption_balances.get((user, asset_sub_id)).try_read().unwrap_or(0)
    }

    #[storage(read)]
    fn get_hook() -> ContractId {
        storage.default_hook.read()
    }

    #[storage(write)]
    fn set_hook(hook: ContractId) {
        require(!hook.is_zero(), "InvalidAddress");
        storage.default_hook.write(hook);
    }
}

impl TokenRouter for Contract {
    #[storage(read)]
    fn router(domain: u32) -> b256 {
        _get_router(domain)
    }

    #[storage(read)]
    fn all_routers() -> Vec<b256> {
        let count = storage.domains.len();
        let mut i = 0;
        let mut routers = Vec::new();

        while i < count {
            if let Some(domain_entry) = storage.domains.get(i) {
                let domain = domain_entry.read();
                if let Some(router) = storage.routers.get(domain).try_read()
                {
                    routers.push(router);
                }
            }
            i += 1;
        }

        routers
    }

    #[storage(read)]
    fn all_domains() -> Vec<u32> {
        let count = storage.domains.len();
        let mut i = 0;
        let mut result = Vec::new();

        while i < count {
            if let Some(domain_entry) = storage.domains.get(i) {
                result.push(domain_entry.read());
            }
            i += 1;
        }

        result
    }

    #[storage(read, write)]
    fn unenroll_remote_router(domain: u32) -> bool {
        let _ = require_authorized(storage.owner.read());

        let removed = storage.routers.remove(domain);

        if removed {
            // Find and remove the domain from domains vec
            let count = storage.domains.len();
            let mut i = 0;

            while i < count {
                if let Some(domain_entry) = storage.domains.get(i) {
                    if domain_entry.read() == domain {
                        let _ = storage.domains.remove(i);
                        return true;
                    }
                }
                i += 1;
            }
        }

        false
    }

    #[storage(read, write)]
    fn enroll_remote_router(domain: u32, router: b256) {
        let _ = require_authorized(storage.owner.read());
        _insert_route_to_state(domain, router);
    }

    #[storage(read, write)]
    fn enroll_remote_routers(domains: Vec<u32>, routers: Vec<b256>) {
        let _ = require_authorized(storage.owner.read());

        require(
            domains
                .len() == routers
                .len(),
            TokenRouterError::RouterLengthMismatch,
        );

        let mut i = 0;
        let length = domains.len();

        while i < length {
            let domain = domains.get(i).unwrap();
            let router = routers.get(i).unwrap();
            _insert_route_to_state(domain, router);
            i += 1;
        }
    }

    #[storage(read)]
    fn remote_router_decimals(router: b256) -> u8 {
        _get_remote_router_decimals(router)
    }

    #[storage(read, write)]
    fn set_remote_router_decimals(router: b256, decimals: u8) {
        let _ = require_authorized(storage.owner.read());
        storage.remote_router_decimals.insert(router, decimals);
    }
}

impl MessageRecipient for Contract {
    #[storage(read)]
    fn handle(origin: u32, sender: b256, message_body: Bytes) {
        let bridge_id = storage.bridge_ids.get(msg_sender().unwrap()).read();
        require(
            storage
                .registered_bridges
                .get(bridge_id)
                .try_read()
                .unwrap_or(false),
            "Sender not a registered bridge",
        );

        let (recipient, amount) = _extract_asset_data_from_body(message_body);
        let recipient_identity = Identity::Address(Address::from(recipient));

        let token_address = sender;

        let sub_id = compute_sub_id(origin.into(), token_address);

        // Check if asset is registered
        require(
            storage
                .asset_parameters
                .get(sub_id)
                .try_read()
                .is_some(),
            "Asset not registered",
        );

        let minter_id = match storage.minter_contract_id.read() {
            Some(id) => id,
            None => {
                require(false, "Minter not set");
                ContractId::from(0x0000000000000000000000000000000000000000000000000000000000000000)
            },
        };

        let minter = abi(WrappedAssetMinter, b256::from(minter_id));

        minter.mint(bridge_id, recipient_identity, sub_id, amount);

        minter.mint_redemption_tickets(Identity::Address(Address::from(recipient)), sub_id, amount);
    }

    #[storage(read)]
    fn extract_asset_data_from_body(message_body: Bytes) {
        let (recipient, amount) = _extract_asset_data_from_body(message_body);

        log(recipient);
        log(amount);
    }

    #[storage(read)]
    fn interchain_security_module() -> ContractId {
        ContractId::from(b256::zero())
    }
}

fn _extract_asset_data_from_body(body: Bytes) -> (b256, u64) {
    let mut buffer_reader = BufferReader::from_parts(body.ptr(), body.len());

    let recipient = buffer_reader.read::<b256>();
    let amount_u256 = buffer_reader.read::<u256>();

    let amount = <u64 as TryFrom<u256>>::try_from(amount_u256).expect("Amount exceeds u64 range");
    (recipient, amount)
}

fn _build_message_body(recipient: b256, amount: u64) -> Bytes {
    let mut buffer = Buffer::new();

    buffer = recipient.abi_encode(buffer);
    let amount_u256 = u256::from(amount);
    buffer = amount_u256.abi_encode(buffer);
    let bytes = Bytes::from(buffer.as_raw_slice());
    bytes
}

// Add these helper functions
#[storage(read)]
fn _get_router(domain: u32) -> b256 {
    storage.routers.get(domain).try_read().unwrap_or(b256::zero())
}

#[storage(read, write)]
fn _insert_route_to_state(domain: u32, router: b256) {
    storage.routers.insert(domain, router);

    // Only add domain to the list if it's not already there
    let count = storage.domains.len();
    let mut i = 0;
    let mut exists = false;

    while i < count {
        if let Some(domain_entry) = storage.domains.get(i) {
            if domain_entry.read() == domain {
                exists = true;
                break;
            }
        }
        i += 1;
    }

    if !exists {
        storage.domains.push(domain);
    }
}

#[storage(read)]
fn _get_remote_router_decimals(router: b256) -> u8 {
    storage.remote_router_decimals.get(router).try_read().unwrap_or(0)
}
