#![no_std]

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use types::{
    DataKey, Milestone, MilestoneStatus, LEDGER_BUMP, LEDGER_THRESHOLD, MAX_DESCRIPTION_LEN,
    MAX_PROOF_URL_LEN, MAX_TITLE_LEN,
};

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, String, Vec};

#[contract]
pub struct MilestoneContract;

#[contractimpl]
impl MilestoneContract {
    /// One-time setup. Not in the original spec: `approve_milestone` and
    /// `reject_milestone` take an `admin: Address` parameter, but without
    /// somewhere to store *the* real admin, that parameter is decorative —
    /// anyone could call those functions passing their own address and
    /// self-authorizing. `initialize` gives the contract an authority to
    /// check the passed `admin` against, mirroring escrow/registry.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .extend_ttl(LEDGER_THRESHOLD, LEDGER_BUMP);
    }

    pub fn create_milestones(
        env: Env,
        creator: Address,
        campaign_id: u64,
        milestones: Vec<(String, String, i128, u32, u32)>,
    ) {
        creator.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::CampaignCreator(campaign_id))
        {
            panic_with_error!(&env, Error::MilestonesAlreadyCreated);
        }
        if milestones.is_empty() {
            panic_with_error!(&env, Error::NoMilestonesProvided);
        }

        let mut ids: Vec<u64> = Vec::new(&env);
        let mut percentage_sum: u32 = 0;
        let now = env.ledger().sequence();

        for (index, (title, description, amount, percentage, deadline_ledger)) in
            milestones.iter().enumerate()
        {
            if title.len() == 0 {
                panic_with_error!(&env, Error::TitleEmpty);
            }
            if title.len() > MAX_TITLE_LEN {
                panic_with_error!(&env, Error::TitleTooLong);
            }
            if description.len() > MAX_DESCRIPTION_LEN {
                panic_with_error!(&env, Error::DescriptionTooLong);
            }
            if amount <= 0 {
                panic_with_error!(&env, Error::InvalidAmount);
            }
            if percentage == 0 || percentage > 100 {
                panic_with_error!(&env, Error::InvalidPercentage);
            }
            if deadline_ledger <= now {
                panic_with_error!(&env, Error::InvalidDeadline);
            }

            percentage_sum = percentage_sum
                .checked_add(percentage)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
            if percentage_sum > 100 {
                panic_with_error!(&env, Error::PercentageSumExceeds100);
            }

            let milestone_id = index as u64;
            let milestone = Milestone {
                id: milestone_id,
                campaign_id,
                title,
                description,
                amount,
                percentage,
                status: MilestoneStatus::Pending,
                deadline_ledger,
                completed_at: None,
                proof_url: String::from_str(&env, ""),
            };
            Self::save_milestone(&env, &milestone);
            ids.push_back(milestone_id);
        }

        let creator_key = DataKey::CampaignCreator(campaign_id);
        env.storage().persistent().set(&creator_key, &creator);
        env.storage()
            .persistent()
            .extend_ttl(&creator_key, LEDGER_THRESHOLD, LEDGER_BUMP);

        let ids_key = DataKey::CampaignMilestoneIds(campaign_id);
        let ids_len = ids.len();
        env.storage().persistent().set(&ids_key, &ids);
        env.storage()
            .persistent()
            .extend_ttl(&ids_key, LEDGER_THRESHOLD, LEDGER_BUMP);

        events::milestones_created(&env, campaign_id, ids_len);
    }

    pub fn submit_milestone(
        env: Env,
        creator: Address,
        campaign_id: u64,
        milestone_id: u64,
        proof_url: String,
    ) {
        creator.require_auth();

        let stored_creator = Self::require_campaign_creator(&env, campaign_id);
        if creator != stored_creator {
            panic_with_error!(&env, Error::NotCampaignCreator);
        }
        if proof_url.len() == 0 {
            panic_with_error!(&env, Error::ProofUrlEmpty);
        }
        if proof_url.len() > MAX_PROOF_URL_LEN {
            panic_with_error!(&env, Error::ProofUrlTooLong);
        }

        let mut milestone = Self::load_milestone(&env, campaign_id, milestone_id);
        match milestone.status {
            MilestoneStatus::Pending | MilestoneStatus::Rejected => {}
            _ => panic_with_error!(&env, Error::InvalidMilestoneStatus),
        }

        milestone.status = MilestoneStatus::UnderReview;
        milestone.proof_url = proof_url;
        Self::save_milestone(&env, &milestone);

        events::milestone_submitted(&env, campaign_id, milestone_id);
    }

    pub fn approve_milestone(env: Env, admin: Address, campaign_id: u64, milestone_id: u64) {
        Self::require_admin_auth(&env, &admin);

        let mut milestone = Self::load_milestone(&env, campaign_id, milestone_id);
        if milestone.status != MilestoneStatus::UnderReview {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }

        milestone.status = MilestoneStatus::Approved;
        Self::save_milestone(&env, &milestone);

        events::milestone_approved(&env, campaign_id, milestone_id);
    }

    pub fn reject_milestone(env: Env, admin: Address, campaign_id: u64, milestone_id: u64) {
        Self::require_admin_auth(&env, &admin);

        let mut milestone = Self::load_milestone(&env, campaign_id, milestone_id);
        if milestone.status != MilestoneStatus::UnderReview {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }

        milestone.status = MilestoneStatus::Rejected;
        Self::save_milestone(&env, &milestone);

        events::milestone_rejected(&env, campaign_id, milestone_id);
    }

    /// Marks a milestone as paid out. Not in the original spec:
    /// `MilestoneStatus::Released` exists, but the spec has no function
    /// that ever transitions a milestone into it. This closes that gap —
    /// the admin/orchestrator calls it right after
    /// `escrow.release_milestone` succeeds for the same milestone.
    pub fn mark_released(env: Env, admin: Address, campaign_id: u64, milestone_id: u64) {
        Self::require_admin_auth(&env, &admin);

        let mut milestone = Self::load_milestone(&env, campaign_id, milestone_id);
        if milestone.status != MilestoneStatus::Approved {
            panic_with_error!(&env, Error::InvalidMilestoneStatus);
        }

        milestone.status = MilestoneStatus::Released;
        milestone.completed_at = Some(env.ledger().sequence());
        Self::save_milestone(&env, &milestone);

        events::milestone_released(&env, campaign_id, milestone_id);
    }

    pub fn get_milestone(env: Env, campaign_id: u64, milestone_id: u64) -> Milestone {
        Self::load_milestone(&env, campaign_id, milestone_id)
    }

    pub fn get_milestones_by_campaign(env: Env, campaign_id: u64) -> Vec<Milestone> {
        let ids = Self::campaign_milestone_ids(&env, campaign_id);
        let mut out = Vec::new(&env);
        for id in ids.iter() {
            out.push_back(Self::load_milestone(&env, campaign_id, id));
        }
        out
    }

    fn require_admin_auth(env: &Env, admin: &Address) {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
        if *admin != stored {
            panic_with_error!(env, Error::Unauthorized);
        }
        admin.require_auth();
    }

    fn require_campaign_creator(env: &Env, campaign_id: u64) -> Address {
        env.storage()
            .persistent()
            .get(&DataKey::CampaignCreator(campaign_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::MilestoneNotFound))
    }

    fn load_milestone(env: &Env, campaign_id: u64, milestone_id: u64) -> Milestone {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(campaign_id, milestone_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::MilestoneNotFound))
    }

    fn save_milestone(env: &Env, milestone: &Milestone) {
        let key = DataKey::Milestone(milestone.campaign_id, milestone.id);
        env.storage().persistent().set(&key, milestone);
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);
    }

    fn campaign_milestone_ids(env: &Env, campaign_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::CampaignMilestoneIds(campaign_id))
            .unwrap_or_else(|| Vec::new(env))
    }
}
