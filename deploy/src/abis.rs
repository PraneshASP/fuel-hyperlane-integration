use fuels::macros::abigen;

abigen!(
    Contract(
        name = "Mailbox",
        abi = "contracts/mailbox/out/debug/mailbox-abi.json",
    ),
    Contract(
        name = "PostDispatch",
        abi = "contracts/mocks/mock-post-dispatch/out/debug/mock-post-dispatch-abi.json",
    ),
    Contract(
        name = "MerkleTreeHook",
        abi = "contracts/hooks/merkle-tree-hook/out/debug/merkle-tree-hook-abi.json",
    ),
    Contract(
        name = "ValidatorAnnounce",
        abi = "contracts/validator-announce/out/debug/validator-announce-abi.json",
    ),
    Contract(
        name = "GasOracle",
        abi = "contracts/gas-oracle/out/debug/gas-oracle-abi.json",
    ),
    Contract(
        name = "GasPaymaster",
        abi = "contracts/hooks/gas-paymaster/out/debug/gas-paymaster-abi.json",
    ),
    Contract(
        name = "TestRecipient",
        abi = "contracts/test/msg-recipient-test/out/debug/msg-recipient-test-abi.json",
    ),
    Contract(
        name = "AggregationISM",
        abi = "contracts/ism/aggregation-ism/out/debug/aggregation-ism-abi.json",
    ),
    Contract(
        name = "DomainRoutingISM",
        abi = "contracts/ism/routing/domain-routing-ism/out/debug/domain-routing-ism-abi.json",
    ),
    Contract(
        name = "FallbackDomainRoutingISM",
        abi = "contracts/ism/routing/default-fallback-domain-routing-ism/out/debug/default-fallback-domain-routing-ism-abi.json",
    ),
    Contract(
        name = "MessageIdMultisigISM",
        abi = "contracts/ism/multisig/message-id-multisig-ism/out/debug/message-id-multisig-ism-abi.json",
    ),
    Contract(
        name = "MerkleRootMultisigISM",
        abi = "contracts/ism/multisig/merkle-root-multisig-ism/out/debug/merkle-root-multisig-ism-abi.json",
    ),
    Contract(
        name = "WarpRoute",
        abi = "contracts/warp-route/out/debug/warp-route-abi.json",
    ),
    Contract(
        name = "SRC20Test",
        abi = "contracts/test/src20-test/out/debug/src20-test-abi.json",
    ),
    Contract(
        name = "ProtocolFee",
        abi = "contracts/hooks/protocol-fee/out/debug/protocol-fee-abi.json",
    ),
    Contract(
        name = "AggregationHook",
        abi = "contracts/hooks/aggregation/out/debug/aggregation-abi.json",
    ),
    Contract(
      name = "PausableHook",
      abi = "contracts/hooks/pausable-hook/out/debug/pausable-hook-abi.json",
    ),
    Contract(
        name = "PausableISM",
        abi = "contracts/ism/pausable-ism/out/debug/pausable-ism-abi.json",
    ),
    Contract(
        name = "FallbackDomainRoutingHook",
        abi = "contracts/hooks/fallback-domain-routing-hook/out/debug/fallback-domain-routing-hook-abi.json",
    ),
    Contract(
        name = "AssetRegistry",
        abi = "contracts/asset-registry/out/debug/asset-registry-abi.json",
    ),
    Contract(
        name = "WrappedAssetMinter",
        abi = "contracts/wrapped-asset-minter/out/debug/wrapped-asset-minter-abi.json",
    )
);
