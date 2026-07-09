use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CampaignStatus {
    Active,
    Funded,
    Expired,
    Cancelled,
    Completed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Campaign {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    pub token: Address,
    pub goal: i128,
    pub raised: i128,
    pub backer_count: u32,
    pub deadline_ledger: u32,
    pub status: CampaignStatus,
    pub milestone_count: u32,
    pub website: String,
    pub image_url: String,
    pub created_at: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    CampaignCount,
    Campaign(u64),
    CreatorCampaigns(Address),
    AllCampaignIds,
}

/// Ledger-count constants used to keep persistent entries alive (rent bumping).
pub const DAY_IN_LEDGERS: u32 = 17_280;
pub const LEDGER_BUMP: u32 = 30 * DAY_IN_LEDGERS;
pub const LEDGER_THRESHOLD: u32 = LEDGER_BUMP - DAY_IN_LEDGERS;

pub const MAX_TITLE_LEN: u32 = 200;
pub const MAX_DESCRIPTION_LEN: u32 = 4_000;
pub const MAX_URL_LEN: u32 = 300;
