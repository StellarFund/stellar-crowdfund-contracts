# Deployments

Contract addresses for each network the project has been deployed to.
Generated with [`scripts/deploy.sh`](scripts/deploy.sh); update this file
after every redeploy.

## Testnet

Deployed via [`scripts/deploy.sh`](scripts/deploy.sh). All 4 contracts are
initialized with the admin address below.

| Contract | Contract ID | Wasm Hash |
|---|---|---|
| campaign | [`CDALT5JO2T2Y7O67HQ5RMBLTLDIT4ZPTV4TPYMFUL2BQJV3P2PWWW4QI`](https://stellar.expert/explorer/testnet/contract/CDALT5JO2T2Y7O67HQ5RMBLTLDIT4ZPTV4TPYMFUL2BQJV3P2PWWW4QI) | `ce15dcd8bfd5c1afe9ce4b7187582756c0a31866ad6f892e551f67e2519cc704` |
| escrow | [`CDLBYX7HRZ2RGJH2V67INPB6YCARXVUTOXAWCFLKGM4ARZSDIM5EHUVC`](https://stellar.expert/explorer/testnet/contract/CDLBYX7HRZ2RGJH2V67INPB6YCARXVUTOXAWCFLKGM4ARZSDIM5EHUVC) | `bfbe9464152feb66146958ded1a500db8b22a0acdce51e3cab4ff5d5a9942f42` |
| milestone | [`CCKKPEBVFFCIAGRJKOXAVOOKS52T2JL5FILBFELNI7SGS36BNBT3HVGB`](https://stellar.expert/explorer/testnet/contract/CCKKPEBVFFCIAGRJKOXAVOOKS52T2JL5FILBFELNI7SGS36BNBT3HVGB) | `5aefc8accf283aa04f3d4f85cc0479de8f5737a2e25cdeb112a5f805eb25741b` |
| registry | [`CDGCZTUJYH2ZSRWBRV6HBKPGY22FFU6BNJ2ULO2GCEOZQTC5D4BLCUJO`](https://stellar.expert/explorer/testnet/contract/CDGCZTUJYH2ZSRWBRV6HBKPGY22FFU6BNJ2ULO2GCEOZQTC5D4BLCUJO) | `3a7a3e032832fc601722b209173023ae57814d693cd78e1bc297af730a4cf2ec` |

- **Network passphrase:** `Test SDF Network ; September 2015`
- **RPC URL:** `https://soroban-testnet.stellar.org`
- **Admin address:** `GDK4JIF23WO2GYNOZUF4AQPM3EZ4BASAUD6XRXZ647LAC565SPZAJBT2`
- **Deployed:** 2026-07-10
- **Deployed with:** `stellar 27.0.0` (`stellar-xdr 27.0.0`)

> **Note on wasm optimization:** these builds were deployed unoptimized —
> the `stellar-cli` binary used here was installed with
> `--no-default-features` (see the `additional-libs`/`libudev` note in
> [CONTRIBUTING.md](CONTRIBUTING.md)), which omits `stellar contract
> optimize`. Wasm sizes are already small (7.4–13.7 KB) since the release
> profile uses `opt-level = "z"` and LTO, so this has no functional impact
> — only a minor, non-load-bearing difference in on-chain footprint size.

## Mainnet

Not yet deployed.
