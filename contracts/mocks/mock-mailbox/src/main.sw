contract;

use std::{bytes::Bytes, hash::{Hash, sha256,},};

abi MessageRecipient {
    #[storage(read, write)]
    fn handle(origin: u32, sender: b256, message_body: Bytes);
    #[storage(read)]
    fn get_bridge_id_storage() -> b256;
}

storage {
    registry: Option<b256> = Option::None,
}

struct DispatchEvent {
    sender: Identity,
    destination_domain: u32,
    recipient: b256,
    message: Bytes,
}

struct DispatchIdEvent {
    message_id: b256,
}

abi MockHyperlaneMailbox {
    #[storage(read, write)]
    fn initialize(registry: b256);

    #[storage(read)]
    fn process(recipient: b256, origin: u32, sender: b256, amount: u64);

    #[storage(read)]
    fn bridge_id() -> b256;

    #[storage(read, write)]
    fn set_registry(registry: b256);

    #[storage(read)]
    fn get_registry() -> Option<b256>;

    #[storage(read)]
    fn dispatch(
        destination_domain: u32,
        recipient: b256,
        message_body: Bytes,
    ) -> b256;
}

impl MockHyperlaneMailbox for Contract {
    #[storage(read, write)]
    fn initialize(registry: b256) {
        require(
            storage
                .registry
                .read() == Option::None,
            "Already initialized",
        );
        storage.registry.write(Some(registry));
    }

    #[storage(read)]
    fn process(recipient: b256, origin: u32, sender: b256, amount: u64) {
        let registry = storage.registry.read().unwrap();
        let recipient_contract = abi(MessageRecipient, registry);
        let message_body = _create_message_body(recipient, amount);
        recipient_contract.handle(origin, sender, message_body);
    }

    #[storage(read)]
    fn bridge_id() -> b256 {
        let registry = storage.registry.read().unwrap();
        let recipient_contract = abi(MessageRecipient, registry);
        recipient_contract.get_bridge_id_storage()
    }

    #[storage(read, write)]
    fn set_registry(registry: b256) {
        storage.registry.write(Some(registry));
    }

    #[storage(read)]
    fn get_registry() -> Option<b256> {
        storage.registry.try_read().unwrap()
    }

    #[storage(read)]
    fn dispatch(
        destination_domain: u32,
        recipient: b256,
        message_body: Bytes,
    ) -> b256 {
        let id = sha256(message_body);

        log(DispatchEvent {
            sender: msg_sender().unwrap(),
            destination_domain: destination_domain,
            recipient: recipient,
            message: message_body,
        });

        log(DispatchIdEvent {
            message_id: id,
        });

        id
    }
}

fn _create_message_body(recipient: b256, amount: u64) -> Bytes {
    let mut buffer = Buffer::new();

    buffer = recipient.abi_encode(buffer);
    let amount_u256 = u256::from(amount); // Convert `u64` to `U256` for 32-byte padding
    buffer = amount_u256.abi_encode(buffer);
    let bytes = Bytes::from(buffer.as_raw_slice());
    bytes
}
