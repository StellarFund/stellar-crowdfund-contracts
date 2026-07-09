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

## Deployments

Testnet contract addresses live in [DEPLOYMENTS.md](DEPLOYMENTS.md).

## Sister repos

- Web app: [StellarFund/stellar-crowdfund-web](https://github.com/StellarFund/stellar-crowdfund-web)
- API + Docs: [StellarFund/stellar-crowdfund-api-docs](https://github.com/StellarFund/stellar-crowdfund-api-docs)

## License

Apache-2.0
