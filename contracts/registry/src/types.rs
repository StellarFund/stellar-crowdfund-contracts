use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryStats {
    pub total_campaigns: u64,
    pub total_raised: i128,
    pub total_backers: u32,
    pub total_completed: u32,
}

/// Internal per-campaign mirror, not part of the public spec. `get_stats`
/// recomputes the platform-wide totals by summing over these on every read
/// rather than maintaining running counters, so there's no drift/replay
/// hazard if `update_stats` is ever called out of order.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignEntry {
    pub campaign_id: u64,
    pub creator: Address,
    pub token: Address,
    pub goal: i128,
    pub raised: i128,
    pub backer_count: u32,
    pub completed: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    AllCampaignIds,
    FeaturedCampaignIds,
    CampaignEntry(u64),
}

pub const DAY_IN_LEDGERS: u32 = 17_280;
pub const LEDGER_BUMP: u32 = 30 * DAY_IN_LEDGERS;
pub const LEDGER_THRESHOLD: u32 = LEDGER_BUMP - DAY_IN_LEDGERS;
