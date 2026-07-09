# Milestone Contract

Tracks each campaign's milestones and the submit → review →
approve/reject/release workflow that gates when escrowed funds are allowed
to move.

Source: [`contracts/milestone/src/lib.rs`](../contracts/milestone/src/lib.rs)

## Status lifecycle

```
                submit_milestone()      approve_milestone()      mark_released()
   ┌─────────┐ ──────────────────► ┌─────────────┐ ──────────────────► ┌──────────┐ ──────────────────► ┌──────────┐
   │ Pending │                     │ UnderReview │                     │ Approved │                     │ Released │
   └─────────┘                     └──────┬──────┘                     └──────────┘                     └──────────┘
        ▲                                 │ reject_milestone()
        │                                 ▼
        │                          ┌──────────┐
        └────submit_milestone()────┤ Rejected │
                                    └──────────┘
```

A rejected milestone can be resubmitted (`Rejected → UnderReview`) with a
new `proof_url`. Every other transition happens exactly once.

## Why this contract has two functions beyond the spec

### `initialize(env, admin)`
The spec gives `approve_milestone`/`reject_milestone` an `admin: Address`
parameter, but this contract has no `initialize` in the spec — so there's
nowhere to check that parameter against. Taken literally, that means
*anyone* could call `approve_milestone`, pass their own address as `admin`,
and have it self-authorize (an address always successfully
`require_auth()`s as itself). `initialize` gives the contract a stored
authority to validate the passed `admin` against before requiring its
auth — the same pattern used in [escrow.md](escrow.md) for
`release_milestone`/`register_campaign`/`enable_refunds`.

### `mark_released(env, admin, campaign_id, milestone_id)`
`MilestoneStatus::Released` exists in the spec's enum, but no spec'd
function ever produces it — `approve_milestone` stops at `Approved`. This
contract has no on-chain link to the escrow contract (consistent with
[escrow.md](escrow.md)'s decoupled design), so it can't know for itself
when `escrow.release_milestone` actually executed. `mark_released` is the
minimal admin-gated function that closes this gap: the orchestrator calls
it right after `escrow.release_milestone` succeeds for the same
`(campaign_id, milestone_id)`, so the milestone's on-chain status reflects
reality.

## Types

### `MilestoneStatus`

| Variant | Meaning |
|---|---|
| `Pending` | Created, creator hasn't submitted proof yet. |
| `UnderReview` | Creator submitted proof, awaiting admin decision. |
| `Approved` | Admin approved; escrow release is expected next. |
| `Rejected` | Admin rejected; creator may resubmit. |
| `Released` | Escrow paid out; terminal. |

### `Milestone`

| Field | Type | Notes |
|---|---|---|
| `id` | `u64` | Index within its campaign, `0`-based, assigned by `create_milestones`. |
| `campaign_id` | `u64` | |
| `title` | `String` | 1–200 chars. |
| `description` | `String` | ≤ 4000 chars. |
| `amount` | `i128` | Must be > 0. Not cross-checked against the campaign's goal — this contract doesn't read campaign state (see below). |
| `percentage` | `u32` | 1–100; the sum across a campaign's milestones must be ≤ 100. |
| `status` | `MilestoneStatus` | See above. |
| `deadline_ledger` | `u32` | Must be in the future at creation time. |
| `completed_at` | `Option<u32>` | Set by `mark_released`. |
| `proof_url` | `String` | Empty until `submit_milestone`; ≤ 300 chars. |

## Functions

### `initialize(env, admin)`
One-time setup, `admin.require_auth()`.

### `create_milestones(env, creator, campaign_id, milestones)`
`creator.require_auth()`. `milestones` is
`Vec<(title, description, amount, percentage, deadline_ledger)>`. Can only
be called once per `campaign_id` (`MilestonesAlreadyCreated` on a second
call — milestones are meant to be set up once, at campaign creation, not
edited afterward). Validates every entry (non-empty title, length limits,
`amount > 0`, `1 <= percentage <= 100`, `deadline_ledger` in the future)
and that percentages sum to ≤ 100 across the whole set. Records `creator`
as the address allowed to submit proof for this campaign's milestones —
this contract doesn't call into `campaign` to verify that address actually
owns the campaign there; the two are wired together by whichever
orchestrator calls both contracts (see [escrow.md](escrow.md) for the same
reasoning applied to `register_campaign`).

### `submit_milestone(env, creator, campaign_id, milestone_id, proof_url)`
`creator.require_auth()`, and `creator` must match the address that called
`create_milestones` for this campaign (`NotCampaignCreator` otherwise).
Legal from `Pending` or `Rejected` only. Sets `proof_url` and moves to
`UnderReview`.

### `approve_milestone(env, admin, campaign_id, milestone_id)`
Admin-gated (see above). Legal only from `UnderReview`.

### `reject_milestone(env, admin, campaign_id, milestone_id)`
Admin-gated. Legal only from `UnderReview`. Creator may `submit_milestone`
again afterward.

### `mark_released(env, admin, campaign_id, milestone_id)`
Admin-gated (see above). Legal only from `Approved`. Sets `completed_at`
to the current ledger sequence.

### `get_milestone(env, campaign_id, milestone_id) -> Milestone`
Read-only. Panics `MilestoneNotFound` for an unknown id.

### `get_milestones_by_campaign(env, campaign_id) -> Vec<Milestone>`
Read-only. Empty vector for a campaign with no milestones created yet.

## Errors

| Code | Variant | Raised when |
|---|---|---|
| 1 | `NotInitialized` | Admin-gated function called before `initialize`. |
| 2 | `AlreadyInitialized` | `initialize` called twice. |
| 3 | `Unauthorized` | Passed `admin` doesn't match the stored admin. |
| 4 | `MilestonesAlreadyCreated` | `create_milestones` called twice for the same campaign. |
| 5 | `NoMilestonesProvided` | `create_milestones` called with an empty vector. |
| 6 | `InvalidAmount` | A milestone's `amount <= 0`. |
| 7 | `InvalidPercentage` | A milestone's `percentage` is 0 or > 100. |
| 8 | `PercentageSumExceeds100` | The set's percentages sum to more than 100. |
| 9 | `InvalidDeadline` | A milestone's `deadline_ledger` isn't in the future. |
| 10 | `TitleEmpty` | A milestone's `title.len() == 0`. |
| 11 | `TitleTooLong` | `title.len() > 200`. |
| 12 | `DescriptionTooLong` | `description.len() > 4000`. |
| 13 | `ProofUrlTooLong` | `submit_milestone`'s `proof_url.len() > 300`. |
| 14 | `ProofUrlEmpty` | `submit_milestone`'s `proof_url.len() == 0`. |
| 15 | `MilestoneNotFound` | Unknown `(campaign_id, milestone_id)`, or campaign has no milestones. |
| 16 | `NotCampaignCreator` | `submit_milestone` caller isn't the registered creator. |
| 17 | `InvalidMilestoneStatus` | Transition attempted from a status that doesn't allow it. |
| 18 | `Overflow` | Defensive; not reachable with realistic percentage values. |

## Events

| Topic | Data | Emitted by |
|---|---|---|
| `created` | milestone count | `create_milestones` |
| `submit` | `milestone_id` | `submit_milestone` |
| `approve` | `milestone_id` | `approve_milestone` |
| `reject` | `milestone_id` | `reject_milestone` |
| `released` | `milestone_id` | `mark_released` |

Every topic tuple is `(topic_symbol, campaign_id)`.

## Storage layout

- **Instance**: `Admin` (`Address`).
- **Persistent**: `CampaignCreator(campaign_id)` → `Address`;
  `CampaignMilestoneIds(campaign_id)` → `Vec<u64>`;
  `Milestone(campaign_id, milestone_id)` → `Milestone`.
