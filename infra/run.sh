#!/bin/bash

# Function to check if port 8545 is in use
check_port_8545() {
    if lsof -i:8545 -t >/dev/null; then
        echo "Port 8545 is already in use. Skipping node startup."
        return 1
    fi
    return 0
}

# Cleanup
kill_processes() {
    if [ -n "$ANVIL_PID" ]; then
        echo "Killing Anvil process with PID $ANVIL_PID"
        kill $ANVIL_PID
    fi

    if [ -n "$FUEL_CORE_PID" ]; then
        echo "Killing Fuel Core process with PID $FUEL_CORE_PID"
        kill $FUEL_CORE_PID
    fi

    if [ -n "$RELAYER_PID" ]; then
        echo "Killing Relayer process with PID $RELAYER_PID"
        kill $RELAYER_PID
    fi

    if [ -n "$VALIDATOR_PID" ]; then
        echo "Killing Validator process with PID $VALIDATOR_PID"
        kill $VALIDATOR_PID
    fi
rm -rf $OUTPUT_PATH
}

trap kill_processes EXIT

# Default values for variables
ENVIRONMENT=""
AGENT=""

# Validation
usage() {
    echo "Usage: $0 --env <LOCAL|TESTNET> --agent <RELAYER|VALIDATOR>"
    exit 1
}
validate_env() {
    if [[ "$ENVIRONMENT" != "LOCAL" && "$ENVIRONMENT" != "TESTNET" ]]; then
        echo "Error: Invalid environment. Only 'LOCAL' or 'TESTNET' are allowed."
        exit 1
    fi
}
validate_agent() {
    if [[ "$AGENT" != "RELAYER" && "$AGENT" != "VALIDATOR" ]]; then
        echo "Error: Invalid agent. Only 'RELAYER' or 'VALIDATOR' are allowed."
        exit 1
    fi
}

# Parse the command-line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --env)
            ENVIRONMENT="$2"
            shift 2
            ;;
        --agent)
            AGENT="$2"
            shift 2
            ;;
        *)
            echo "Unknown parameter: $1"
            usage
            ;;
    esac
done

if [[ -z "$ENVIRONMENT" || -z "$AGENT" ]]; then
    echo "Error: Both --env and --agent arguments are required."
    usage
fi

validate_env
validate_agent

echo "Setting up infrastructure for $ENVIRONMENT."

# Paths
PROJECT_ROOT=$(git rev-parse --show-toplevel)
INFRA_PATH="$PROJECT_ROOT/infra"
OUTPUT_PATH="$INFRA_PATH/output"
MONOREPO_PATH="$INFRA_PATH/hyperlane-monorepo"
ENV_FILE="$INFRA_PATH/.env"

load_env_file() {
    if [ -f "$ENV_FILE" ]; then
        source "$ENV_FILE"
    else
        echo "Error: .env file not found in $INFRA_PATH. Exiting..."
        exit 1
    fi
}

check_env_var() {
    local var_name="$1"
    if [ -z "${!var_name}" ]; then
        echo "Error: $var_name is not set or empty. Exiting..."
        exit 1
    fi
}

# Load .env file and check required variables
load_env_file
required_vars=("FUEL_SIGNER_KEY" "SEPOLIA_SIGNER_KEY" "SEPOLIA_SIGNER_ADDRESS")
for var in "${required_vars[@]}"; do
    check_env_var "$var"
done
echo "All required environment variables are set."

# Check if monorepo exists, clone if not
if [ ! -d "$MONOREPO_PATH" ]; then
    echo "Monorepo not found. Cloning..."
    git clone --branch feat/fuel-integration --single-branch https://github.com/fuel-infrastructure/hyperlane-monorepo "$MONOREPO_PATH"
else
    echo "Monorepo found."
fi


ANVIL_PID=""
FUEL_CORE_PID=""

if [ "$ENVIRONMENT" == "LOCAL" ]; then
    # Paths
    FUEL_LOCAL_SNAPSHOT="$INFRA_PATH/configs/local-fuel-snapshot"
    FUNDED_ANVIL_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    export HYP_KEY="$FUNDED_ANVIL_PRIVATE_KEY"
    ANVIL_OUTPUT="$OUTPUT_PATH/nodes/anvil_output.log"
    FUEL_CORE_OUTPUT="$OUTPUT_PATH/nodes/fuelcore_output.log"
    HYP_CLI_CORE_CONFIGS="$INFRA_PATH/configs/core-config.yaml"
    HYP_CLI_WR_NATIVE_CONFIGS="$INFRA_PATH/configs/warp-route-native.yaml"
    HYP_CLI_WR_COLLATERAL_CONFIGS="$INFRA_PATH/configs/warp-route-collateral.yaml"
    HYP_CLI_WR_SYNTHETIC_CONFIGS="$INFRA_PATH/configs/warp-route-synthetic.yaml"
    LOCAL_FUEL_CONTRACT_DUMP="$OUTPUT_PATH/contracts"

    mkdir -p "$OUTPUT_PATH/nodes"
    touch "$ANVIL_OUTPUT" "$FUEL_CORE_OUTPUT"

    # Update YAML configuration for contract owner
    update_core_config() {
        local key="$1"
        local value="$2"
        yq eval ".$key = \"$value\"" -i "$HYP_CLI_CORE_CONFIGS"
    }

    update_wr_configs() {
        local mailbox_address="$1"
        local owner_address="$2"
        yq e ".test1.mailbox = \"$mailbox_address\"" "$HYP_CLI_WR_NATIVE_CONFIGS" -i
        yq e ".test1.owner = \"$owner_address\"" "$HYP_CLI_WR_NATIVE_CONFIGS" -i

        yq e ".test1.mailbox = \"$mailbox_address\"" "$HYP_CLI_WR_COLLATERAL_CONFIGS" -i
        yq e ".test1.owner = \"$owner_address\"" "$HYP_CLI_WR_COLLATERAL_CONFIGS" -i

        yq e ".test1.mailbox = \"$mailbox_address\"" "$HYP_CLI_WR_SYNTHETIC_CONFIGS" -i
        yq e ".test1.owner = \"$owner_address\"" "$HYP_CLI_WR_SYNTHETIC_CONFIGS" -i
    }
    
    echo "Updating hyperlane deployment configurations with Sepolia signer address..."
    update_core_config "owner" "$SEPOLIA_SIGNER_ADDRESS"
    update_core_config "requiredHook.owner" "$SEPOLIA_SIGNER_ADDRESS"

    start_anvil() {
        echo "Starting Anvil node..."
        anvil &> "$ANVIL_OUTPUT" &
        ANVIL_PID=$!
        echo "Anvil is running with PID $ANVIL_PID"
    }

    start_fuel_core() {
        echo "Starting Fuel Core node..."
        fuel-core run --db-type in-memory --debug --snapshot "$FUEL_LOCAL_SNAPSHOT" &> "$FUEL_CORE_OUTPUT" &
        FUEL_CORE_PID=$!
        echo "Fuel Core is running with PID $FUEL_CORE_PID"
    }

    wait_for_log() {
        local logfile="$1"
        local pattern="$2"
        while ! grep -q "$pattern" "$logfile"; do
            sleep 0.5
        done
    }

   if ! lsof -i:8545 -t >/dev/null; then
    echo "Port 8545 is not in use. Proceeding with node startup."
    
    # Start local nodes
    start_anvil
    start_fuel_core

    echo "Waiting for nodes to be ready..."
    wait_for_log "$ANVIL_OUTPUT" "Listening on 127.0.0.1:8545"
    wait_for_log "$FUEL_CORE_OUTPUT" "Starting GraphQL_Off_Chain_Worker service"

    # Deploy Hyperlane Core and contracts
    echo "Deploying Hyperlane Core..."
    ANVIL_DEPLOYMENT_DUMP="$INFRA_PATH/configs/chains/test1/addresses.yaml"
    LOG_LEVEL="TRACE"  hyperlane core deploy --private-key "$HYP_KEY" -y --chain test1 --overrides "$INFRA_PATH/configs" --config "$HYP_CLI_CORE_CONFIGS"

    ANVIL_MAILBOX_ADDRESS=$(yq e ".mailbox" "$ANVIL_DEPLOYMENT_DUMP")
    update_wr_configs "$ANVIL_MAILBOX_ADDRESS" "$SEPOLIA_SIGNER_ADDRESS"

    echo "Deploying Hyperlane Collateral Warp Route..."
    hyperlane warp deploy --overrides "$INFRA_PATH/configs" -y --config "$HYP_CLI_WR_COLLATERAL_CONFIGS"
    echo "Deploying Hyperlane Synthetic Warp Route..."
    hyperlane warp deploy --overrides "$INFRA_PATH/configs" -y --config "$HYP_CLI_WR_SYNTHETIC_CONFIGS"
    echo "Deploying Hyperlane Native Warp Route..."
    hyperlane warp deploy --overrides "$INFRA_PATH/configs" -y --config "$HYP_CLI_WR_NATIVE_CONFIGS"

    echo "Deploying FuelVM contracts..."
    cd "$PROJECT_ROOT/deploy" && RUSTFLAGS="-Awarnings" cargo run -- LOCAL "test" "$LOCAL_FUEL_CONTRACT_DUMP"

    # Write deployments to configs
    echo "Writing deployments to configs..."
    LOCAL_CONFIG_FILE="$INFRA_PATH/configs/agent-config-local.json"

    LOCAL_FUEL_CONTRACT_DUMP_FULL="$LOCAL_FUEL_CONTRACT_DUMP/local/contract_addresses.yaml"

    # Paths to contract dumps and config file
    LOCAL_FUEL_KEYS=("mailbox" "postDispatch" "testRecipient" "interIdSecurityModule" "merkleTreeHook" "interchainGasPaymaster" "validatorAnnounce" "gasOracle" "aggregationISM" "domainRoutingISM" "fallbackDomainRoutingISM" "messageIdMultisigISM1" "merkleRootMultisigISM1" "messageIdMultisigISM3" "merkleRootMultisigISM3" "warpRouteNative" "warpRouteSynthetic" "warpRouteCollateral" "collateralTokenContract" "testCollateralAsset" "aggregationHook" "pausableHook" "protocolFee")
    LOCAL_ANVIL_KEYS=("domainRoutingIsmFactory" "interchainAccountIsm" "interchainAccountRouter" "mailbox" "proxyAdmin" "staticAggregationHookFactory" "staticAggregationIsmFactory" "staticMerkleRootMultisigIsmFactory" "staticMessageIdMultisigIsmFactory" "testRecipient" "validatorAnnounce")

    # mkdir "$PROJECT_ROOT/infra/output/contracts"
    # mkdir "$PROJECT_ROOT/infra/output/contracts/local"
    touch "$PROJECT_ROOT/infra/output/contracts/chains/test1/addresses.yaml"
    # Read fuel data
    declare -A FUEL_VALUES
    for key in "${LOCAL_FUEL_KEYS[@]}"; do
        FUEL_VALUES[$key]=$(yq e ".$key" "$LOCAL_FUEL_CONTRACT_DUMP_FULL")
    done

    # Read anvil data
    declare -A ANVIL_VALUES
    for key in "${LOCAL_ANVIL_KEYS[@]}"; do
        ANVIL_VALUES[$key]=$(yq e ".$key" "$ANVIL_DEPLOYMENT_DUMP")
    done

    # Write fuel data to config file
    for key in "${LOCAL_FUEL_KEYS[@]}"; do
        yq e ".chains.fueltest1.$key = \"${FUEL_VALUES[$key]}\"" "$LOCAL_CONFIG_FILE" -i
    done

    # Write anvil data to config file
    for key in "${LOCAL_ANVIL_KEYS[@]}"; do
        yq e ".chains.test1.$key = \"${ANVIL_VALUES[$key]}\"" "$LOCAL_CONFIG_FILE" -i
    done
    
    else
        echo "Port 8545 is already in use. Skipping node startup."

    fi
fi



# Paths and settings
LOG_PATH=""
DB_DIR=""
RELAY_CHAINS=""
VALIDATOR_CHAIN=""
FUEL_CHAIN_NAME=""
EVM_CHAIN_NAME=""
mkdir -p "$OUTPUT_PATH/agents"

# Environment-specific settings
set_environment_config() {
    if [ "$ENVIRONMENT" == "TESTNET" ]; then
        FUEL_CHAIN_NAME="fueltestnet"
        EVM_CHAIN_NAME="basesepolia"
        export CONFIG_FILES="$INFRA_PATH/configs/agent-config.json"
    else
        FUEL_CHAIN_NAME="fueltest1"
        EVM_CHAIN_NAME="test1"
        export CONFIG_FILES="$INFRA_PATH/configs/agent-config-local.json"

        # TODO fix this temp workaround if needed
        export GASPAYMENTENFORCEMENT="[{\"type\": \"none\"}]"
        export HYP_CHAINS_TEST1_MERKLETREEHOOK="0xB7f8BC63BbcaD18155201308C8f3540b07f84F5e"
        export HYP_CHAINS_TEST1_INTERCHAINGASPAYMASTER="0x0000000000000000000000000000000000000000"
    fi
}

# Function to set common variables for agents
set_common_agent_vars() {
    LOG_PATH="$OUTPUT_PATH/agents/$1.log"
    RELAY_CHAINS="$FUEL_CHAIN_NAME,$EVM_CHAIN_NAME"
    VALIDATOR_CHAIN="$FUEL_CHAIN_NAME"
    DB_DIR="$OUTPUT_PATH/agents/hyperlane_db_$1_$VALIDATOR_CHAIN"
    touch "$LOG_PATH"
}

# Function to run relayer
run_relayer() {
    cargo run --release --bin relayer -- \
        --db "$DB_DIR" \
        --relayChains "$RELAY_CHAINS" \
        --allowLocalCheckpointSyncers true \
        --defaultSigner.key "$FUEL_SIGNER_KEY" \
        --chains.$FUEL_CHAIN_NAME.signer.key "$FUEL_SIGNER_KEY" \
        --chains.$EVM_CHAIN_NAME.signer.key "$SEPOLIA_SIGNER_KEY" \
        --metrics-port 9091 \
        --blacklist '[{"recipient_address": ["0x0c140f16e63bc541d6f6d7c128e6c0630dcb4e12a3b9f42128036d29aee5f8e8","0x3cdbb5772fcde1b804203cc2db2a6038e1abd16057f767aba1ba063cf148b5aa","0x0aa0f26609e68739b94f470d290f61629c21e9d1728f84642cc4f5ad29830f4d","0x93328d66b51486c6fe863b8fa4fd8396b4868a31507c8dbefb0174fcb022029b","0xdbece6d3eb12041ecd71e098ff00b186d687fbf73441a42b7f7b8dd5261efef5","0x76315239d9ad8f4e84744552af18e932ee6bc585faea5d863ff8413bb264aabc","0x1262dd920f1ff668a9b5afd7bf80a4cc84bc44e9d41d47bc212312d65dd21d9f","0x987b9073e10ac826a0409e5cb84db16eb86a6464f18bc404106fa9b2ab698488","0x80fa7fb4e7639697859578824a4513cd21b15354fbae3b295dfe6411ac416d73","0x3237ed56a0797cf4d7d2b22fc08efc5743b12941c25440f14fd6ca039ef3ec13","0x5a61f4c2b23b46347744bcae7b8bb51d56aba96d2992a247d1b9a6c3c2b0f9c8","0x0b5b9fc13018dfdf858fbbdfefa4f355cbb5b2f97738240be20d08ffcea95b77","0x3d2a842ca0bef63caa52652f75d0de98bbd966e3a157a2f3513ff4d392d7e189","0xcf84455834df6f3d5c7d66c652d1efe9b7f6b76d2c2424490304c5e3c77de350", "0xe56858e529c13e6e20e62307cfb5cbd44adbf49e82b94f51af146fb83bc5265b", "0xe34ef8b05d710f68734175f0098c963d282be55d8bd74b6c490b3270bb074da3", "0x631bc085813a20760c531728c44282c5c654d1591cd7f9389bc7c4b48b4730c7", "0xeb3893b282542ba1cdea6882741189ac5c11c3ac58e3a6521cfaa5b6d03139d1", "0xbb255cf4e418c26f66ac56d0f0584c47c5220d2fe4beabe1948a33c52f75a39e", "0x827eed329bcf70de612af4cfb55ed56c5f6a3f9ddd76bc679632da1e3f10f3d7", "0x008189b24262b6b1416e78f5431f4bb7134e27ac4857e47433949ecaefc766fa", "0x0c8cb2bb74390290441857bbfce16d78307dbebf40e4f707b2898c39e39e0250", "0x69a3eed1094a2470e5f011edabc63eb94d1a1ff9181b0e65da1ab96cf1fc088f", "0xc97b10747544a4588f7ee852393245c62d14453f0310e1f16aad72b228e4ea45", "0xcc3dffdd37c134c908a13af2cc3a67eb3f9b483c3b1560d5511169317e25c602", "0xa347fa1775198aa68fb1a4523a4925f891cca8f4dc79bf18ca71274c49f600c3", "0xed9f3fe51cafde8a47afaa50cddc00c83fc4976f7b03c715179593c9119e2aae", "0x3660ccbf57f3bb04545626f66c9b05a2d5502f5d9b59317f7d465d10e8e71b26", "0xcc2f05bb978c1d2c15034d2c35bce2b99c7b78c301d7713e2fb4ba52e60c3496"]}]' \
        > "$LOG_PATH" 2>&1 &
    RELAYER_PID=$!
    tail -f "$LOG_PATH"
}

# Function to run validator
run_validator() {
    export VALIDATOR_SIGNATURES_DIR="$OUTPUT_PATH/hyperlane-validator-signatures-$VALIDATOR_CHAIN"
    export HYP_DEFAULTSIGNER_KEY="$FUEL_SIGNER_KEY"

    cargo run --release --bin validator -- \
        --db "$DB_DIR" \
        --originChainName "$VALIDATOR_CHAIN" \
        --checkpointSyncer.type localStorage \
        --checkpointSyncer.path "$VALIDATOR_SIGNATURES_DIR" \
        --validator.key "$SEPOLIA_SIGNER_KEY" \
        > "$LOG_PATH" 2>&1 &
    VALIDATOR_PID=$!
    tail -f "$LOG_PATH"
}

# Main logic
set_environment_config
cd "$MONOREPO_PATH/rust/main"

if [ "$AGENT" == "RELAYER" ]; then
    set_common_agent_vars "relayer"
    run_relayer
else [ "$AGENT" == "VALIDATOR" ]; 
    set_common_agent_vars "validator"
    run_validator
fi


