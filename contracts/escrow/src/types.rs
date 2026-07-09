use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contribution {
    pub backer: Address,
    pub campaign_id: u64,
    pub amount: i128,
    pub token: Address,
    pub ledger: u32,
    pub refunded: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowState {
    pub campaign_id: u64,
    pub token: Address,
    pub total_locked: i128,
    pub total_released: i128,
    pub total_refunded: i128,
    pub contributions: soroban_sdk::Vec<Contribution>,
}

/// Internal bookkeeping record, one per campaign. `creator` and `refundable`
/// are not part of the public `EscrowState` spec — they exist purely to let
/// this contract operate without cross-contract calls into `campaign` (see
/// docs/escrow.md for why).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowMeta {
    pub campaign_id: u64,
    pub creator: Address,
    pub token: Address,
    pub total_locked: i128,
    pub total_released: i128,
    pub total_refunded: i128,
    pub refundable: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    EscrowMeta(u64),
    Contribution(u64, Address),
    CampaignBackers(u64),
    BackerCampaigns(Address),
}

pub const DAY_IN_LEDGERS: u32 = 17_280;
pub const LEDGER_BUMP: u32 = 30 * DAY_IN_LEDGERS;
pub const LEDGER_THRESHOLD: u32 = LEDGER_BUMP - DAY_IN_LEDGERS;
