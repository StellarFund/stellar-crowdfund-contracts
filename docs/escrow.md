# Escrow Contract

Custodies backer contributions per campaign and pays them out — either to
the campaign creator, as milestones are approved, or back to backers, if the
campaign expires or is cancelled.

Source: [`contracts/escrow/src/lib.rs`](../contracts/escrow/src/lib.rs)

## Why this contract has two functions beyond the spec

`release_milestone(admin, campaign_id, milestone_id, amount)` has to send
`amount` of the campaign's token to *someone* — the campaign creator. But
neither `release_milestone` nor `contribute` carries a recipient/creator
address, and this contract deliberately does not make on-chain
cross-contract calls into `campaign` to look one up:

- Soroban's idiomatic way to call a sibling contract is
  `soroban_sdk::contractimport!`, which needs that contract's *compiled
  wasm* to exist at **escrow's own compile time** — including for plain
  native `cargo test`. That imposes a fragile build order (campaign must be
  built to wasm before escrow can even be unit tested) that breaks easily on
  a fresh clone or in CI.
- The alternative — adding `campaign` as a normal path dependency so escrow
  can use `campaign::CampaignContractClient` directly — pulls campaign's
  entire `#[contractimpl]`-generated code (all its `#[no_mangle]` wasm
  exports) into escrow's own compiled wasm binary, since both crates need
  `crate-type = ["cdylib", "rlib"]`. Deployed `escrow.wasm` would then
  accidentally also expose `create_campaign`, `cancel_campaign`, etc.

Instead, escrow keeps a small mirror of exactly the two facts it needs per
campaign — the creator's payout address and the accepted token — via two
admin-gated functions:

### `register_campaign(env, admin, campaign_id, creator, token)`
Called once by the admin/orchestrator right after
`campaign.create_campaign` succeeds (typically in response to watching that
contract's `create` event). Stores an `EscrowMeta` record for the campaign.
Must not be called twice for the same `campaign_id`.

### `enable_refunds(env, admin, campaign_id)`
Called once by the admin/orchestrator after observing (via
`campaign.get_campaign`) that a campaign transitioned to `Expired` or
`Cancelled`. Flips an internal `refundable` flag, which is what
`refund_all` and `refund_backer` gate on. Without this, escrow has no
independent way to know a campaign is refund-eligible — trusting the
admin's flag here plays the same role as the admin gate on campaign's own
`mark_expired`/`mark_funded`.

Neither function changes the public `EscrowState`/`Contribution` shape —
`creator` and `refundable` live only in the internal `EscrowMeta` record,
not in anything returned to callers.

## Types

### `Contribution`
One record per `(backer, campaign_id)` pair — **not** one row per deposit.
Contributing twice to the same campaign accumulates into the same record
(`amount` sums, `ledger` stays at the first contribution's ledger sequence,
i.e. "when this backer entered").

| Field | Type |
|---|---|
| `backer` | `Address` |
| `campaign_id` | `u64` |
| `amount` | `i128` — cumulative across all `contribute` calls |
| `token` | `Address` |
| `ledger` | `u32` — ledger sequence of the *first* contribution |
| `refunded` | `bool` |

### `EscrowState`
Returned by `get_escrow_state`. `contributions` is assembled on read from
every backer who has contributed to the campaign.

| Field | Type |
|---|---|
| `campaign_id` | `u64` |
| `token` | `Address` |
| `total_locked` | `i128` — cumulative contributions, never decreases |
| `total_released` | `i128` — cumulative milestone releases |
| `total_refunded` | `i128` — cumulative refunds |
| `contributions` | `Vec<Contribution>` |

`total_locked - total_released - total_refunded` is the amount still held
by the contract for that campaign.

## Functions

### `initialize(env, admin)`
One-time setup, `admin.require_auth()`.

### `register_campaign(env, admin, campaign_id, creator, token)`
See above. Admin-gated: the passed `admin` must equal the stored admin
*and* authorize the call — passing an arbitrary address as `admin` and
having it self-authorize is not enough. Panics `CampaignAlreadyRegistered`
on a duplicate `campaign_id`.

### `enable_refunds(env, admin, campaign_id)`
See above. Admin-gated the same way. Panics `RefundsAlreadyEnabled` if
called twice.

### `contribute(env, backer, campaign_id, amount, token)`
`backer.require_auth()`, then transfers `amount` of `token` from `backer`
to the escrow contract via the standard token client. Requires:
- the campaign is registered (`CampaignNotRegistered` otherwise),
- `token` matches the token the campaign registered with
  (`TokenMismatch` otherwise),
- `amount > 0` (`InvalidAmount` otherwise),
- refunds haven't been enabled for this campaign yet (`CampaignRefundable`
  otherwise — once a campaign is refund-eligible, new money shouldn't come
  in).

Adds to the backer's `Contribution.amount` and `EscrowState.total_locked`.

### `release_milestone(env, admin, campaign_id, milestone_id, amount)`
Admin-gated. Transfers `amount` of the campaign's token from escrow to the
registered creator. Requires `amount > 0` and `amount <=
total_locked - total_released - total_refunded` (`InsufficientFunds`
otherwise). `milestone_id` is not validated against the milestone
contract — it's carried through purely for the event/audit trail; the
admin/orchestrator is expected to only call this after seeing the
corresponding milestone approved (see [milestone.md](milestone.md)).

### `refund_all(env, campaign_id)`
**Not** admin-gated — matches the spec's signature, which has no
authorizing address at all. Safe to leave open because the actual
authorization already happened in `enable_refunds`: this function only
pays out exactly each backer's own recorded `Contribution.amount`, and
skips anyone already refunded, so there's no way to call it for harm.
Panics `RefundsNotEnabled` if `enable_refunds` hasn't run yet. A no-op
(succeeds, refunds nothing) if there are no backers.

### `refund_backer(env, backer, campaign_id)`
Self-service single refund. `backer.require_auth()`. Requires refunds are
enabled and the backer hasn't already been refunded
(`AlreadyRefunded`/`ContributionNotFound` otherwise).

### `get_escrow_state(env, campaign_id) -> EscrowState`
Read-only. Panics `CampaignNotRegistered` for an unknown campaign.

### `get_contribution(env, backer, campaign_id) -> Option<Contribution>`
Read-only. Returns `None` rather than panicking if the backer never
contributed — this one genuinely needs `Option` since "no contribution" is
an expected, common case, not an error.

### `get_contributions_by_campaign(env, campaign_id) -> Vec<Contribution>`
### `get_contributions_by_backer(env, backer) -> Vec<Contribution>`
Read-only, empty vector if none.

## Errors

| Code | Variant | Raised when |
|---|---|---|
| 1 | `NotInitialized` | Admin-gated function called before `initialize`. |
| 2 | `AlreadyInitialized` | `initialize` called twice. |
| 3 | `Unauthorized` | Passed `admin` address doesn't match the stored admin. |
| 4 | `CampaignAlreadyRegistered` | `register_campaign` called twice for the same id. |
| 5 | `CampaignNotRegistered` | Any campaign-scoped call before `register_campaign`. |
| 6 | `InvalidAmount` | `amount <= 0` in `contribute`/`release_milestone`. |
| 7 | `TokenMismatch` | `contribute`'s `token` doesn't match the registered token. |
| 8 | `InsufficientFunds` | `release_milestone` amount exceeds what's still held. |
| 9 | `ContributionNotFound` | Refund requested for a backer with no contribution. |
| 10 | `AlreadyRefunded` | `refund_backer` called twice for the same backer. |
| 11 | `RefundsNotEnabled` | Refund attempted before `enable_refunds`. |
| 12 | `RefundsAlreadyEnabled` | `enable_refunds` called twice. |
| 13 | `CampaignRefundable` | `contribute` attempted after refunds were enabled. |
| 14 | `Overflow` | Defensive; not reachable with realistic token amounts. |

## Events

| Topic | Data | Emitted by |
|---|---|---|
| `register` | `(creator, token)` | `register_campaign` |
| `contrib` | `(backer, amount)` | `contribute` |
| `release` | `(milestone_id, amount)` | `release_milestone` |
| `refunden` | `()` | `enable_refunds` |
| `refundal` | `total_refunded` | `refund_all` |
| `refundbk` | `(backer, amount)` | `refund_backer` |

Every topic tuple is `(topic_symbol, campaign_id)`.

## Storage layout

- **Instance**: `Admin` (`Address`).
- **Persistent**: `EscrowMeta(campaign_id)`; `Contribution(campaign_id,
  backer)`; `CampaignBackers(campaign_id)` → `Vec<Address>`;
  `BackerCampaigns(backer)` → `Vec<u64>`.
