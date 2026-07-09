# Campaign Contract

Manages the full lifecycle of a crowdfunding campaign: creation, cancellation,
and the state-machine transitions that mark a campaign as funded, expired, or
completed.

Source: [`contracts/campaign/src/lib.rs`](../contracts/campaign/src/lib.rs)

## Status lifecycle

```
                 goal reached            all milestones released
   ┌────────┐   mark_funded()   ┌────────┐   mark_completed()   ┌───────────┐
   │ Active │ ────────────────► │ Funded │ ────────────────────► │ Completed │
   └────────┘                   └────────┘                       └───────────┘
     │    │
     │    │ deadline passed, goal not reached
     │    └────────────────────► ┌─────────┐
     │        mark_expired()     │ Expired │
     │                           └─────────┘
     │ creator cancels
     └───────────────────────────► ┌───────────┐
              cancel_campaign()    │ Cancelled │
                                   └───────────┘
```

`Active` is the only status from which a campaign can transition. Every other
status is terminal.

## Why this contract has an `initialize` step

The published spec for this contract does not thread an admin/authority
parameter through `update_raised`, `mark_funded`, `mark_expired`, or
`mark_completed` — those are system-level transitions driven by the escrow
contract (after a contribution or milestone release) rather than by the
campaign creator or an individual backer. Without *some* authority check,
anyone could call `update_raised` with a fabricated amount and force a
campaign into `Funded`/`Completed` state.

To close that gap without changing any of the given function signatures, the
contract stores an `admin` `Address` (set once via `initialize`, mirroring the
pattern already used by the escrow and registry contracts) and calls
`admin.require_auth()` internally inside those four functions. The admin
address isn't a parameter — it's read from contract storage — so the public
API surface described in the spec is unchanged. In production this admin key
is held by whatever process orchestrates the escrow/milestone contracts (see
[escrow.md](escrow.md) and [milestone.md](milestone.md)).

Creator-facing functions (`create_campaign`, `cancel_campaign`) are gated by
`creator.require_auth()` instead, and are never affected by the admin.

## Types

### `CampaignStatus`

| Variant | Meaning |
|---|---|
| `Active` | Accepting contributions, deadline not yet reached. |
| `Funded` | Goal reached; escrow releases funds per approved milestone. |
| `Expired` | Deadline passed without reaching the goal; backers may claim refunds. |
| `Cancelled` | Creator cancelled while still `Active`; backers may claim refunds. |
| `Completed` | All milestones released; campaign lifecycle finished. |

### `Campaign`

| Field | Type | Notes |
|---|---|---|
| `id` | `u64` | Sequential, assigned by `create_campaign`. |
| `creator` | `Address` | Must authorize `create_campaign`/`cancel_campaign`. |
| `title` | `String` | 1–200 chars. |
| `description` | `String` | Up to 4000 chars. |
| `token` | `Address` | SAC/asset contract accepted for contributions. |
| `goal` | `i128` | Must be > 0. |
| `raised` | `i128` | Running total, updated via `update_raised`. |
| `backer_count` | `u32` | Incremented once per `update_raised` call. |
| `deadline_ledger` | `u32` | Must be greater than the ledger sequence at creation. |
| `status` | `CampaignStatus` | See above. |
| `milestone_count` | `u32` | Must be ≥ 1; informational cache of the milestone contract's data. |
| `website` / `image_url` | `String` | Optional, ≤ 300 chars each. |
| `created_at` | `u32` | Ledger sequence at creation. |

## Functions

### `initialize(env, admin: Address)`
One-time setup. Requires `admin.require_auth()`. Panics with
`AlreadyInitialized` if called twice.

### `create_campaign(...) -> u64`
Requires `creator.require_auth()`. Validates `goal > 0`, `deadline_ledger` is
in the future, `milestone_count >= 1`, and string length limits. Returns the
new campaign's `id`. Emits a `create` event keyed by campaign id, `(creator,
goal)`.

### `cancel_campaign(env, creator, campaign_id)`
Requires `creator.require_auth()` **and** that `creator` matches the stored
`Campaign.creator`. Only legal while `status == Active`. Emits a `cancel`
event.

### `get_campaign(env, campaign_id) -> Campaign`
Read-only. Panics with `CampaignNotFound` if the id doesn't exist.

### `get_campaigns_by_creator(env, creator) -> Vec<Campaign>`
Read-only. Returns an empty vector if the creator has no campaigns.

### `get_active_campaigns(env) -> Vec<Campaign>`
Read-only. Filters the full campaign set down to `status == Active`.

### `update_raised(env, campaign_id, amount)`
Admin-only (see above). Requires `amount > 0` and `status == Active`. Adds
`amount` to `raised` and increments `backer_count` by 1 per call (it counts
contribution events, not unique backers — see [escrow.md](escrow.md) for how
unique-backer accounting works). Emits a `raised` event with `(amount,
new_total)`.

### `mark_funded(env, campaign_id)`
Admin-only. Requires `status == Active` and `raised >= goal`. Emits a
`funded` event.

### `mark_expired(env, campaign_id)`
Admin-only. Requires `status == Active` and the current ledger sequence to be
`>= deadline_ledger`. Emits an `expired` event.

### `mark_completed(env, campaign_id)`
Admin-only. Requires `status == Funded`. Emits a `complete` event.

## Errors

All errors are typed (`#[contracterror]`) and raised via `panic_with_error!`
rather than `unwrap()`/`panic!()`, so failures always surface as a specific,
decodable error code instead of an opaque trap.

| Code | Variant | Raised when |
|---|---|---|
| 1 | `NotInitialized` | Admin-gated function called before `initialize`. |
| 2 | `AlreadyInitialized` | `initialize` called more than once. |
| 3 | `CampaignNotFound` | Unknown `campaign_id`. |
| 4 | `NotCampaignCreator` | Caller isn't the campaign's creator. |
| 5 | `InvalidGoal` | `goal <= 0`. |
| 6 | `InvalidDeadline` | `deadline_ledger` not in the future. |
| 7 | `InvalidMilestoneCount` | `milestone_count == 0`. |
| 8 | `CampaignNotActive` | Transition attempted from a non-`Active` status. |
| 9 | `CampaignNotFunded` | `mark_completed` called on a non-`Funded` campaign. |
| 10 | `InvalidAmount` | `update_raised` called with `amount <= 0`. |
| 11 | `TitleTooLong` | `title.len() > 200`. |
| 12 | `TitleEmpty` | `title.len() == 0`. |
| 13 | `DescriptionTooLong` | `description.len() > 4000`. |
| 14 | `UrlTooLong` | `website`/`image_url` `.len() > 300`. |
| 15 | `DeadlineNotReached` | `mark_expired` called before the deadline ledger. |
| 16 | `GoalNotReached` | `mark_funded` called before `raised >= goal`. |
| 17 | `Overflow` | Arithmetic overflow on `raised`/`backer_count` (defensive; not reachable with realistic values). |

## Events

| Topic | Data | Emitted by |
|---|---|---|
| `create` | `(creator, goal)` | `create_campaign` |
| `cancel` | `creator` | `cancel_campaign` |
| `raised` | `(amount, total_raised)` | `update_raised` |
| `funded` | `()` | `mark_funded` |
| `expired` | `()` | `mark_expired` |
| `complete` | `()` | `mark_completed` |

Every event's topic tuple is `(topic_symbol, campaign_id)`, so consumers can
filter by campaign without decoding the payload.

## Storage layout

- **Instance**: `Admin` (`Address`), `CampaignCount` (`u64`).
- **Persistent**: `Campaign(id)` → `Campaign`; `CreatorCampaigns(creator)` →
  `Vec<u64>`; `AllCampaignIds` → `Vec<u64>`.

Persistent entries have their TTL extended on every write (30-day bump once
the remaining TTL drops below ~29 days) so campaign data doesn't expire from
archival state between contributions.
