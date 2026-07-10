# Stellar Crowdfund Contracts

Soroban smart contracts powering **StellarFund**, a milestone-based
crowdfunding platform on Stellar. Project creators set a funding goal and a
series of milestones; backers contribute XLM or any Stellar asset; funds sit
in an on-chain escrow and are released to the creator only as milestones are
verified — never as a single lump sum on funding.

Part of the [StellarFund](https://github.com/StellarFund) GitHub org.

## Architecture

Four independent Soroban contracts, one per Cargo workspace member:

```
┌───────────────┐        ┌───────────────┐
│    Registry    │◄──────┤    Campaign    │
│  platform-wide │        │  lifecycle &   │
│  index + stats │        │  metadata      │
└───────────────┘        └───────┬───────┘
                                   │ raised / status
                                   │ updates (admin)
                          ┌───────▼───────┐        ┌────────────────┐
                          │     Escrow     │◄──────►│    Milestone    │
                          │  holds funds,  │  amount │  proof review   │
                          │  releases per  │  on     │  & approval     │
                          │  milestone     │  approve│  workflow       │
                          └───────────────┘        └────────────────┘
```

- **[Campaign](contracts/campaign)** — creates campaigns, tracks status
  (`Active → Funded/Expired/Cancelled → Completed`), and is the source of
  truth for goal/raised/deadline. See [docs/campaign.md](docs/campaign.md).
- **[Escrow](contracts/escrow)** — custodies backer contributions per
  campaign and releases funds to the creator as milestones are approved, or
  refunds backers if a campaign expires or is cancelled. See
  [docs/escrow.md](docs/escrow.md).
- **[Milestone](contracts/milestone)** — tracks each campaign's milestones
  and the submit → review → approve/reject workflow creators and the
  platform admin use to unlock escrowed funds. See
  [docs/milestone.md](docs/milestone.md).
- **[Registry](contracts/registry)** — a lightweight, platform-wide index of
  every campaign plus aggregate stats, so a frontend can list/feature
  campaigns without walking the campaign contract's storage directly. See
  [docs/registry.md](docs/registry.md).

These contracts intentionally don't make cross-contract calls to each other
on-chain — each one's admin-gated, system-driven functions (`update_raised`,
`release_milestone`, `register_campaign`, etc.) are meant to be invoked by a
trusted off-chain orchestrator (or, in a later iteration, wired together via
direct cross-contract calls) that watches events from one contract and drives
the next. See each contract's doc page for its specific auth model.

## Tech stack

- Rust + [Soroban SDK](https://developers.stellar.org/docs/build/smart-contracts) 22.x
- Cargo workspace, one crate per contract

## Getting started

See [CONTRIBUTING.md](CONTRIBUTING.md) for local dev setup, build/test
commands, and how to deploy to testnet.

```sh
cargo build
cargo test
```

## Stellar integration

This repo is the on-chain half of StellarFund — everything below is what the
[web app](https://github.com/StellarFund/stellar-crowdfund-web) and the
[API indexer](https://github.com/StellarFund/stellar-crowdfund-api) actually
build against.

### Network

| | Testnet | Mainnet |
|---|---|---|
| Passphrase | `Test SDF Network ; September 2015` | `Public Global Stellar Network ; September 2015` |
| Soroban RPC | `https://soroban-testnet.stellar.org` | — |
| Horizon | `https://horizon-testnet.stellar.org` | — |
| Status | **Deployed** (see below) | Not yet deployed |

### Deployed contracts (testnet)

Redeployed via [`scripts/deploy.sh`](scripts/deploy.sh); full detail (wasm
hashes, CLI version, admin key) in [DEPLOYMENTS.md](DEPLOYMENTS.md), always
the source of truth if this table drifts.

| Contract | Contract ID |
|---|---|
| `campaign` | [`CDALT5JO2T2Y7O67HQ5RMBLTLDIT4ZPTV4TPYMFUL2BQJV3P2PWWW4QI`](https://stellar.expert/explorer/testnet/contract/CDALT5JO2T2Y7O67HQ5RMBLTLDIT4ZPTV4TPYMFUL2BQJV3P2PWWW4QI) |
| `escrow` | [`CDLBYX7HRZ2RGJH2V67INPB6YCARXVUTOXAWCFLKGM4ARZSDIM5EHUVC`](https://stellar.expert/explorer/testnet/contract/CDLBYX7HRZ2RGJH2V67INPB6YCARXVUTOXAWCFLKGM4ARZSDIM5EHUVC) |
| `milestone` | [`CCKKPEBVFFCIAGRJKOXAVOOKS52T2JL5FILBFELNI7SGS36BNBT3HVGB`](https://stellar.expert/explorer/testnet/contract/CCKKPEBVFFCIAGRJKOXAVOOKS52T2JL5FILBFELNI7SGS36BNBT3HVGB) |
| `registry` | [`CDGCZTUJYH2ZSRWBRV6HBKPGY22FFU6BNJ2ULO2GCEOZQTC5D4BLCUJO`](https://stellar.expert/explorer/testnet/contract/CDGCZTUJYH2ZSRWBRV6HBKPGY22FFU6BNJ2ULO2GCEOZQTC5D4BLCUJO) |

All four are **fixed, singleton instances** — there is no per-campaign
contract deployment. Every campaign is just a `u64` ID inside these four
contracts' storage; `campaign.create_campaign(...)` returns that ID, which
callers then pass to every other contract (`escrow.contribute(backer,
campaign_id, ...)`, `milestone.submit_milestone(creator, campaign_id,
milestone_id, ...)`, etc.). If you're integrating against this repo, don't
expect (or ask a contract for) a per-campaign contract address — there isn't
one.

### On-chain events

Every state-changing function publishes an event keyed `(topic, campaign_id)`
so an off-chain indexer can reconstruct state without polling every contract
function. Topics are short symbols (`symbol_short!`, ≤9 chars), not the
longer descriptive names you might expect:

| Contract | Topic | Data | Emitted by |
|---|---|---|---|
| `campaign` | `create` | `(creator, goal)` | `create_campaign` |
| `campaign` | `cancel` | `creator` | `cancel_campaign` |
| `campaign` | `raised` | `(amount, total_raised)` | `update_raised` |
| `campaign` | `funded` | `()` | `mark_funded` |
| `campaign` | `expired` | `()` | `mark_expired` |
| `campaign` | `complete` | `()` | `mark_completed` |
| `escrow` | `register` | `(creator, token)` | `register_campaign` |
| `escrow` | `contrib` | `(backer, amount)` | `contribute` |
| `escrow` | `release` | `(milestone_id, amount)` | `release_milestone` |
| `escrow` | `refunden` | `()` | `enable_refunds` |
| `escrow` | `refundal` | `total_refunded` | `refund_all` |
| `escrow` | `refundbk` | `(backer, amount)` | `refund_backer` |
| `milestone` | `created` | `count` | `create_milestones` |
| `milestone` | `submit` | `milestone_id` | `submit_milestone` |
| `milestone` | `approve` | `milestone_id` | `approve_milestone` |
| `milestone` | `reject` | `milestone_id` | `reject_milestone` |
| `milestone` | `released` | `milestone_id` | `mark_released` |
| `registry` | `register` | `(creator, goal)` | `register_campaign` |
| `registry` | `stats` | `(raised, backer_count)` | `update_stats` |
| `registry` | `complete` | `()` | `mark_campaign_completed` |
| `registry` | `featured` | `featured` | `set_featured` |

Source: `contracts/*/src/events.rs` in each contract crate.

### The off-chain orchestrator gap

Because the four contracts don't call each other on-chain (see
[Architecture](#architecture) above), *something* has to watch events from
one and drive the admin-gated functions on the others — e.g. call
`escrow.register_campaign` right after `campaign.create_campaign`, or
`registry.update_stats` + `campaign.update_raised` right after
`escrow.contribute`. **That orchestrator does not exist yet in any repo in
this org.** `scripts/deploy.sh` sets each contract's `admin` to the same
deployer identity, so today this wiring has to be done by hand (or scripted
ad hoc) per campaign. If you're picking up this work:

- The [API repo](https://github.com/StellarFund/stellar-crowdfund-api)'s
  indexer already polls Horizon and decodes Soroban events on a 10s
  interval — it's the natural place to add the write side, but as of this
  writing it's read-only and its assumed event schema/architecture (a
  registry that deploys one contract per campaign) predates this repo's
  real design and needs to be reconciled first.
- The admin key needs to move from a single CLI identity to whatever
  service runs the orchestrator, with the security implications that
  implies — see each contract doc's auth model section before doing this
  (e.g. [campaign.md](docs/campaign.md#why-this-contract-has-an-initialize-step)).

### Who talks to these contracts

| Repo | Role | How |
|---|---|---|
| [`stellar-crowdfund-web`](https://github.com/StellarFund/stellar-crowdfund-web) | Read + write, from a connected wallet | `@stellar/stellar-sdk` (simulate for reads, build/sign/submit for writes) + Freighter for signing. **Not yet wired to this repo's real function names/IDs** — see that repo's README. |
| [`stellar-crowdfund-api`](https://github.com/StellarFund/stellar-crowdfund-api) | Read-only index | Polls Horizon, decodes Soroban events into a queryable REST API. **Assumes a different (outdated) contract design** — see that repo's README. |

## Deployments

Full testnet contract addresses, wasm hashes, and deploy metadata live in
[DEPLOYMENTS.md](DEPLOYMENTS.md).

## Sister repos

- Web app: [StellarFund/stellar-crowdfund-web](https://github.com/StellarFund/stellar-crowdfund-web)
- API + Docs: [StellarFund/stellar-crowdfund-api](https://github.com/StellarFund/stellar-crowdfund-api)

## License

Apache-2.0
