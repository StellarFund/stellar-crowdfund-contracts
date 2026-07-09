# Contributing

## Local dev setup

1. **Install Rust** via [rustup](https://rustup.rs):
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. **Add the Soroban wasm target:**
   ```sh
   rustup target add wasm32v1-none
   ```
   (Older toolchains/SDK versions use `wasm32-unknown-unknown` instead — if
   `stellar contract build` complains about the target, add that one too.)
3. **Install the Stellar CLI** (needed for testnet deploys and for
   `stellar contract build`, which wraps `cargo build` with the right flags):
   ```sh
   cargo install --locked stellar-cli
   ```
   This compiles from source and needs a C toolchain (`build-essential` /
   Xcode command line tools) on your machine.
4. **Clone and build:**
   ```sh
   git clone https://github.com/StellarFund/stellar-crowdfund-contracts.git
   cd stellar-crowdfund-contracts
   cargo build
   ```

### A note on `Cargo.lock` and `ed25519-dalek`

`Cargo.lock` is committed (these crates compile to deployable wasm, so
reproducible builds matter) — do not delete it to "fix" dependency issues.
In particular, `soroban-env-host` declares `ed25519-dalek >= 2.0.0` with no
upper bound, so an unconstrained resolve can pick up a future breaking major
release of `ed25519-dalek` and fail to compile `soroban-env-host`'s own test
utilities with a `CryptoRng`/`rand_core` trait-bound error. If you ever need
to regenerate the lockfile and hit that error, pin back to the last known-good
2.x release:

```sh
cargo update -p ed25519-dalek@<version-that-got-picked> --precise 2.2.0
```

## How to build

```sh
cargo build
```

To produce the optimized wasm binaries used for deployment (one per
contract, under `target/wasm32v1-none/release/`):

```sh
stellar contract build
```

## How to test

Run the full workspace test suite:

```sh
cargo test
```

Run a single contract's tests:

```sh
cargo test -p campaign
cargo test -p escrow
cargo test -p milestone
cargo test -p registry
```

Each contract's tests live in `contracts/<name>/src/test.rs` and are compiled
only under `#[cfg(test)]`. Every public function has at least one happy-path
test, one unauthorized-caller test, and coverage for the edge cases that
apply to it (double-init, double-refund, zero/negative amounts, expired
deadlines, etc). New functions should follow the same pattern — see
"Adding a new contract function" below.

## How to deploy to testnet

1. Create and fund a deployer identity (one-time):
   ```sh
   stellar keys generate --global crowdfund-deployer --network testnet --fund
   ```
2. Run the deploy script:
   ```sh
   ./scripts/deploy.sh testnet crowdfund-deployer
   ```
   This builds all 4 contracts, uploads and deploys each one, initializes
   campaign/escrow/registry with the deployer as admin (milestone has no
   `initialize` step — its admin-gated functions take the admin address as an
   explicit parameter), and prints a summary table.
3. Paste the resulting addresses into [DEPLOYMENTS.md](DEPLOYMENTS.md).

To deploy a single contract manually instead of using the script:

```sh
stellar contract build
stellar contract deploy \
  --wasm target/wasm32v1-none/release/campaign.wasm \
  --source crowdfund-deployer \
  --network testnet
```

## How to add a new contract function

1. Add the function to the relevant `contracts/<name>/src/lib.rs`, inside the
   `#[contractimpl] impl ... Contract` block.
2. Call `require_auth()` on whichever `Address` is supposed to authorize the
   call — the creator/backer for user-initiated actions, or the stored/
   passed admin `Address` for system-driven state transitions. Never skip
   this for a function that mutates state on someone else's behalf.
3. Validate inputs and fail with a typed error via `panic_with_error!(&env,
   Error::YourVariant)` — add the variant to `errors.rs` if none fits. Don't
   use `unwrap()`, `expect()`, or bare `panic!()`.
4. If the function changes stored state, emit an event for it (see
   `events.rs` in the relevant contract) and extend the TTL of any
   persistent storage entry you write to.
5. Add tests to `src/test.rs`: at minimum a happy path, and — if the
   function has an auth or validation requirement — a `#[should_panic]` test
   for each failure mode you introduced.
6. Update the corresponding doc in `docs/<name>.md` (function reference,
   error table, event table).
7. Run `cargo test -p <name>` and `cargo build` before committing.

## Sister repos

- Web app: [StellarFund/stellar-crowdfund-web](https://github.com/StellarFund/stellar-crowdfund-web)
- API + Docs: [StellarFund/stellar-crowdfund-api-docs](https://github.com/StellarFund/stellar-crowdfund-api-docs)
