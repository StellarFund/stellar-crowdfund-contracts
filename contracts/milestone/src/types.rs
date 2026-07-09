use soroban_sdk::{contracttype, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
    Released,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u64,
    pub campaign_id: u64,
    pub title: String,
    pub description: String,
    pub amount: i128,
    pub percentage: u32,
    pub status: MilestoneStatus,
    pub deadline_ledger: u32,
    pub completed_at: Option<u32>,
    pub proof_url: String,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    /// campaign_id -> the Address that called create_milestones for it.
    /// Needed to authorize submit_milestone, since this contract has no
    /// other way to know who owns a campaign's milestones.
    CampaignCreator(u64),
    /// campaign_id -> Vec<u64> of milestone ids, in creation order.
    CampaignMilestoneIds(u64),
    /// (campaign_id, milestone_id) -> Milestone.
    Milestone(u64, u64),
}

pub const DAY_IN_LEDGERS: u32 = 17_280;
pub const LEDGER_BUMP: u32 = 30 * DAY_IN_LEDGERS;
pub const LEDGER_THRESHOLD: u32 = LEDGER_BUMP - DAY_IN_LEDGERS;

pub const MAX_TITLE_LEN: u32 = 200;
pub const MAX_DESCRIPTION_LEN: u32 = 4_000;
pub const MAX_PROOF_URL_LEN: u32 = 300;
