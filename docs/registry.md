# Registry Contract

A lightweight, platform-wide index of every campaign plus aggregate stats,
so a frontend can list/feature campaigns and show platform totals without
walking the campaign contract's storage directly.

Source: [`contracts/registry/src/lib.rs`](../contracts/registry/src/lib.rs)

This contract intentionally knows very little about each campaign — just
`creator`, `token`, `goal`, and a mirror of `raised`/`backer_count`/
`completed` kept in sync by the admin/orchestrator. For anything else
(title, description, deadline, status), a client is expected to call
`campaign.get_campaign(id)` directly. See [campaign.md](campaign.md).

## Why this contract has one function beyond the spec

`RegistryStats.total_completed` exists in the spec, but no spec'd function
ever increments it — `update_stats` only carries `raised`/`backer_count`.
`mark_campaign_completed(env, campaign_id)` closes that gap: the
admin/orchestrator calls it right after `campaign.mark_completed`
succeeds, the same relationship `escrow.enable_refunds` and
`milestone.mark_released` have with campaign's own state transitions
elsewhere in this project (see [escrow.md](escrow.md) and
[milestone.md](milestone.md)).

## How stats are computed

`get_stats` does **not** maintain running counters that get incremented on
every `update_stats`/`register_campaign` call. Instead it recomputes
`total_raised`, `total_backers`, and `total_completed` by summing over
every registered campaign's mirrored entry on each read. This trades a bit
of read-time cost (O(number of campaigns)) for eliminating an entire class
of drift bugs: `update_stats` can be called with a campaign's latest
absolute `raised`/`backer_count` at any time, in any order, any number of
times, and `get_stats` is always consistent with the current mirrored
state — there's no delta bookkeeping to get wrong.

`total_backers` is a **sum of each campaign's `backer_count`**, not a
count of unique backer addresses platform-wide — it inherits this from
`campaign.backer_count`, which itself counts contribution events rather
than unique backers (see [campaign.md](campaign.md)).

## Types

### `RegistryStats`

| Field | Type | Notes |
|---|---|---|
| `total_campaigns` | `u64` | Count of registered campaigns. |
| `total_raised` | `i128` | Sum of every campaign's mirrored `raised`. |
| `total_backers` | `u32` | Sum of every campaign's mirrored `backer_count` (not deduplicated). |
| `total_completed` | `u32` | Count of campaigns marked completed via `mark_campaign_completed`. |

## Functions

### `initialize(env, admin)`
One-time setup, `admin.require_auth()`.

### `register_campaign(env, campaign_id, creator, token, goal)`
No address parameter in the spec, so — like campaign's own
`update_raised`/`mark_funded`/etc. — authorization comes from the admin
stored at `initialize`, not a passed-in parameter. Requires `goal > 0`.
Panics `CampaignAlreadyRegistered` on a duplicate `campaign_id`.

### `update_stats(env, campaign_id, raised, backer_count)`
Same admin-from-storage authorization as `register_campaign`. Overwrites
the campaign's mirrored `raised`/`backer_count` with the given values
(these are absolute current totals, not deltas — mirror whatever
`campaign.get_campaign(id).raised`/`.backer_count` currently read).
Requires the campaign to already be registered
(`CampaignNotRegistered` otherwise).

### `mark_campaign_completed(env, campaign_id)`
Same admin-from-storage authorization. See above. Panics
`AlreadyCompleted` if called twice for the same campaign.

### `get_all_campaigns(env) -> Vec<u64>`
Read-only. Every registered campaign id, in registration order.

### `get_stats(env) -> RegistryStats`
Read-only. See "How stats are computed" above.

### `get_featured_campaigns(env) -> Vec<u64>`
Read-only. Empty vector if nothing is featured.

### `set_featured(env, admin, campaign_id, featured)`
Admin-gated: the passed `admin` must equal the stored admin *and*
authorize the call. Requires the campaign to already be registered.
Idempotent — setting `featured=true` on an already-featured campaign, or
`featured=false` on one that isn't featured, is a harmless no-op rather
than an error or a duplicate list entry.

## Errors

| Code | Variant | Raised when |
|---|---|---|
| 1 | `NotInitialized` | Admin-gated function called before `initialize`. |
| 2 | `AlreadyInitialized` | `initialize` called twice. |
| 3 | `Unauthorized` | `set_featured`'s passed `admin` doesn't match the stored admin. |
| 4 | `CampaignAlreadyRegistered` | `register_campaign` called twice for the same id. |
| 5 | `CampaignNotRegistered` | `update_stats`/`mark_campaign_completed`/`set_featured` on an unknown id. |
| 6 | `InvalidGoal` | `register_campaign`'s `goal <= 0`. |
| 7 | `InvalidAmount` | `update_stats`'s `raised < 0`, or aggregate overflow in `get_stats` (defensive; not reachable with realistic values). |
| 8 | `AlreadyCompleted` | `mark_campaign_completed` called twice. |

## Events

| Topic | Data | Emitted by |
|---|---|---|
| `register` | `(creator, goal)` | `register_campaign` |
| `stats` | `(raised, backer_count)` | `update_stats` |
| `complete` | `()` | `mark_campaign_completed` |
| `featured` | `featured: bool` | `set_featured` |

Every topic tuple is `(topic_symbol, campaign_id)`.

## Storage layout

- **Instance**: `Admin` (`Address`).
- **Persistent**: `AllCampaignIds` → `Vec<u64>`; `FeaturedCampaignIds` →
  `Vec<u64>`; `CampaignEntry(campaign_id)` → `CampaignEntry` (internal
  mirror: creator, token, goal, raised, backer_count, completed).
