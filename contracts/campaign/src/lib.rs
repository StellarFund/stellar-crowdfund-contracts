#![no_std]

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use types::{
    Campaign, CampaignStatus, DataKey, LEDGER_BUMP, LEDGER_THRESHOLD, MAX_DESCRIPTION_LEN,
    MAX_TITLE_LEN, MAX_URL_LEN,
};

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, String, Vec};

#[contract]
pub struct CampaignContract;

#[contractimpl]
impl CampaignContract {
    /// One-time setup. `admin` is the authority allowed to drive system-level
    /// state transitions (`update_raised`, `mark_funded`, `mark_expired`,
    /// `mark_completed`) — typically the escrow/milestone contracts' operator
    /// or an off-chain relayer key. Creator-initiated actions (create/cancel)
    /// never require the admin.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::CampaignCount, &0u64);
        env.storage()
            .instance()
            .extend_ttl(LEDGER_THRESHOLD, LEDGER_BUMP);
    }

    pub fn create_campaign(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        token: Address,
        goal: i128,
        deadline_ledger: u32,
        milestone_count: u32,
        website: String,
        image_url: String,
    ) -> u64 {
        creator.require_auth();

        if goal <= 0 {
            panic_with_error!(&env, Error::InvalidGoal);
        }
        if deadline_ledger <= env.ledger().sequence() {
            panic_with_error!(&env, Error::InvalidDeadline);
        }
        if milestone_count == 0 {
            panic_with_error!(&env, Error::InvalidMilestoneCount);
        }
        if title.len() == 0 {
            panic_with_error!(&env, Error::TitleEmpty);
        }
        if title.len() > MAX_TITLE_LEN {
            panic_with_error!(&env, Error::TitleTooLong);
        }
        if description.len() > MAX_DESCRIPTION_LEN {
            panic_with_error!(&env, Error::DescriptionTooLong);
        }
        if website.len() > MAX_URL_LEN || image_url.len() > MAX_URL_LEN {
            panic_with_error!(&env, Error::UrlTooLong);
        }

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CampaignCount)
            .unwrap_or(0u64);

        let campaign = Campaign {
            id,
            creator: creator.clone(),
            title,
            description,
            token,
            goal,
            raised: 0,
            backer_count: 0,
            deadline_ledger,
            status: CampaignStatus::Active,
            milestone_count,
            website,
            image_url,
            created_at: env.ledger().sequence(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Campaign(id), &campaign);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Campaign(id), LEDGER_THRESHOLD, LEDGER_BUMP);

        env.storage()
            .instance()
            .set(&DataKey::CampaignCount, &(id + 1));

        let mut creator_ids = Self::creator_ids(&env, &creator);
        creator_ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::CreatorCampaigns(creator.clone()), &creator_ids);
        env.storage().persistent().extend_ttl(
            &DataKey::CreatorCampaigns(creator.clone()),
            LEDGER_THRESHOLD,
            LEDGER_BUMP,
        );

        let mut all_ids = Self::all_ids(&env);
        all_ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::AllCampaignIds, &all_ids);
        env.storage().persistent().extend_ttl(
            &DataKey::AllCampaignIds,
            LEDGER_THRESHOLD,
            LEDGER_BUMP,
        );

        env.storage()
            .instance()
            .extend_ttl(LEDGER_THRESHOLD, LEDGER_BUMP);

        events::campaign_created(&env, id, &creator, goal);

        id
    }

    pub fn cancel_campaign(env: Env, creator: Address, campaign_id: u64) {
        creator.require_auth();

        let mut campaign = Self::load_campaign(&env, campaign_id);
        if campaign.creator != creator {
            panic_with_error!(&env, Error::NotCampaignCreator);
        }
        if campaign.status != CampaignStatus::Active {
            panic_with_error!(&env, Error::CampaignNotActive);
        }

        campaign.status = CampaignStatus::Cancelled;
        Self::save_campaign(&env, &campaign);

        events::campaign_cancelled(&env, campaign_id, &creator);
    }

    pub fn get_campaign(env: Env, campaign_id: u64) -> Campaign {
        Self::load_campaign(&env, campaign_id)
    }

    pub fn get_campaigns_by_creator(env: Env, creator: Address) -> Vec<Campaign> {
        let ids = Self::creator_ids(&env, &creator);
        let mut out = Vec::new(&env);
        for id in ids.iter() {
            out.push_back(Self::load_campaign(&env, id));
        }
        out
    }

    pub fn get_active_campaigns(env: Env) -> Vec<Campaign> {
        let ids = Self::all_ids(&env);
        let mut out = Vec::new(&env);
        for id in ids.iter() {
            let campaign = Self::load_campaign(&env, id);
            if campaign.status == CampaignStatus::Active {
                out.push_back(campaign);
            }
        }
        out
    }

    pub fn update_raised(env: Env, campaign_id: u64, amount: i128) {
        let admin = Self::require_admin(&env);
        admin.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        let mut campaign = Self::load_campaign(&env, campaign_id);
        if campaign.status != CampaignStatus::Active {
            panic_with_error!(&env, Error::CampaignNotActive);
        }

        campaign.raised = campaign
            .raised
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
        campaign.backer_count = campaign
            .backer_count
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));

        Self::save_campaign(&env, &campaign);

        events::raised_updated(&env, campaign_id, amount, campaign.raised);
    }

    pub fn mark_funded(env: Env, campaign_id: u64) {
        let admin = Self::require_admin(&env);
        admin.require_auth();

        let mut campaign = Self::load_campaign(&env, campaign_id);
        if campaign.status != CampaignStatus::Active {
            panic_with_error!(&env, Error::CampaignNotActive);
        }
        if campaign.raised < campaign.goal {
            panic_with_error!(&env, Error::GoalNotReached);
        }

        campaign.status = CampaignStatus::Funded;
        Self::save_campaign(&env, &campaign);

        events::campaign_funded(&env, campaign_id);
    }

    pub fn mark_expired(env: Env, campaign_id: u64) {
        let admin = Self::require_admin(&env);
        admin.require_auth();

        let mut campaign = Self::load_campaign(&env, campaign_id);
        if campaign.status != CampaignStatus::Active {
            panic_with_error!(&env, Error::CampaignNotActive);
        }
        if env.ledger().sequence() < campaign.deadline_ledger {
            panic_with_error!(&env, Error::DeadlineNotReached);
        }

        campaign.status = CampaignStatus::Expired;
        Self::save_campaign(&env, &campaign);

        events::campaign_expired(&env, campaign_id);
    }

    pub fn mark_completed(env: Env, campaign_id: u64) {
        let admin = Self::require_admin(&env);
        admin.require_auth();

        let mut campaign = Self::load_campaign(&env, campaign_id);
        if campaign.status != CampaignStatus::Funded {
            panic_with_error!(&env, Error::CampaignNotFunded);
        }

        campaign.status = CampaignStatus::Completed;
        Self::save_campaign(&env, &campaign);

        events::campaign_completed(&env, campaign_id);
    }

    fn require_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
    }

    fn load_campaign(env: &Env, campaign_id: u64) -> Campaign {
        env.storage()
            .persistent()
            .get(&DataKey::Campaign(campaign_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::CampaignNotFound))
    }

    fn save_campaign(env: &Env, campaign: &Campaign) {
        env.storage()
            .persistent()
            .set(&DataKey::Campaign(campaign.id), campaign);
        env.storage().persistent().extend_ttl(
            &DataKey::Campaign(campaign.id),
            LEDGER_THRESHOLD,
            LEDGER_BUMP,
        );
    }

    fn creator_ids(env: &Env, creator: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::CreatorCampaigns(creator.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn all_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::AllCampaignIds)
            .unwrap_or_else(|| Vec::new(env))
    }
}
