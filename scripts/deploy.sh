#!/usr/bin/env bash
#
# Build and deploy all Stellar Crowdfund contracts to a Soroban network
# (defaults to testnet) using the `stellar` CLI, then print/record the
# resulting contract IDs.
#
# Usage:
#   ./scripts/deploy.sh [network] [source-account]
#
#   network        Network alias known to `stellar keys`/`stellar contract`
#                   (default: testnet)
#   source-account  Name of the identity in `stellar keys` used to pay for
#                   and authorize deployment (default: crowdfund-deployer)
#
# Requirements:
#   - `stellar` CLI installed (cargo install --locked stellar-cli)
#   - `rustup target add wasm32v1-none`
#   - The source account funded on the target network (testnet: friendbot)
#
# The script is idempotent-ish: it always rebuilds and redeploys fresh
# contract instances (Soroban has no "upgrade in place" for these simple
# contracts), and prints a summary you can paste into DEPLOYMENTS.md.

set -euo pipefail

NETWORK="${1:-testnet}"
SOURCE_ACCOUNT="${2:-crowdfund-deployer}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_DIR="${REPO_ROOT}/target/wasm32v1-none/release"

CONTRACTS=(campaign escrow milestone registry)

log() { printf '\n\033[1;34m==>\033[0m %s\n' "$1"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

command -v stellar >/dev/null 2>&1 || die "stellar CLI not found. Install with: cargo install --locked stellar-cli"

if ! stellar keys address "${SOURCE_ACCOUNT}" >/dev/null 2>&1; then
  die "Identity '${SOURCE_ACCOUNT}' not found. Create it first, e.g.:
  stellar keys generate --global ${SOURCE_ACCOUNT} --network ${NETWORK} --fund"
fi

log "Deploying as '${SOURCE_ACCOUNT}' ($(stellar keys address "${SOURCE_ACCOUNT}")) on '${NETWORK}'"

log "Building all contracts (wasm32v1-none, release profile)"
cd "${REPO_ROOT}"
stellar contract build

declare -A CONTRACT_IDS
declare -A WASM_HASHES

for name in "${CONTRACTS[@]}"; do
  wasm_path="${WASM_DIR}/${name}.wasm"
  [ -f "${wasm_path}" ] || die "Expected wasm not found at ${wasm_path} — did the build succeed?"

  log "Optimizing ${name}.wasm"
  optimized_path="${WASM_DIR}/${name}.optimized.wasm"
  if stellar contract optimize --wasm "${wasm_path}" 2>/dev/null && [ -f "${optimized_path}" ]; then
    :
  else
    echo "  (optimize unavailable in this stellar-cli build — deploying unoptimized wasm)"
    optimized_path="${wasm_path}"
  fi

  log "Installing ${name} wasm on ${NETWORK}"
  wasm_hash=$(stellar contract upload \
    --wasm "${optimized_path}" \
    --source "${SOURCE_ACCOUNT}" \
    --network "${NETWORK}")
  WASM_HASHES[$name]="${wasm_hash}"

  log "Deploying ${name} contract instance"
  contract_id=$(stellar contract deploy \
    --wasm-hash "${wasm_hash}" \
    --source "${SOURCE_ACCOUNT}" \
    --network "${NETWORK}")
  CONTRACT_IDS[$name]="${contract_id}"

  echo "  ${name}: ${contract_id}"
done

log "Initializing campaign, escrow, milestone, and registry (admin = ${SOURCE_ACCOUNT})"
ADMIN_ADDRESS=$(stellar keys address "${SOURCE_ACCOUNT}")

for name in campaign escrow milestone registry; do
  stellar contract invoke \
    --id "${CONTRACT_IDS[$name]}" \
    --source "${SOURCE_ACCOUNT}" \
    --network "${NETWORK}" \
    -- initialize --admin "${ADMIN_ADDRESS}"
done

log "Deployment summary"
echo "Network: ${NETWORK}"
echo "Admin:   ${ADMIN_ADDRESS}"
for name in "${CONTRACTS[@]}"; do
  printf '%-10s wasm_hash=%s contract_id=%s\n' "${name}" "${WASM_HASHES[$name]}" "${CONTRACT_IDS[$name]}"
done

log "Done. Paste the table above into DEPLOYMENTS.md."
