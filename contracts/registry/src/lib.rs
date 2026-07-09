#![no_std]

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use types::{CampaignEntry, DataKey, RegistryStats, LEDGER_BUMP, LEDGER_THRESHOLD};

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, Vec};

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
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

    /// Like campaign's own system-transition functions, `register_campaign`
    /// has no address parameter in the spec, so authorization comes from
    /// the admin stored at `initialize` rather than a passed-in parameter.
    /// Called by the admin/orchestrator right after
    /// `campaign.create_campaign` succeeds.
    pub fn register_campaign(env: Env, campaign_id: u64, creator: Address, token: Address, goal: i128) {
        let admin = Self::require_admin(&env);
        admin.require_auth();

        if goal <= 0 {
            panic_with_error!(&env, Error::InvalidGoal);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::CampaignEntry(campaign_id))
        {
            panic_with_error!(&env, Error::CampaignAlreadyRegistered);
        }

        let entry = CampaignEntry {
            campaign_id,
            creator: creator.clone(),
            token,
            goal,
            raised: 0,
            backer_count: 0,
            completed: false,
        };
        Self::save_entry(&env, &entry);

        let mut ids = Self::all_ids(&env);
        ids.push_back(campaign_id);
        let ids_key = DataKey::AllCampaignIds;
        env.storage().persistent().set(&ids_key, &ids);
        env.storage()
            .persistent()
            .extend_ttl(&ids_key, LEDGER_THRESHOLD, LEDGER_BUMP);

        events::campaign_registered(&env, campaign_id, &creator, goal);
    }

    pub fn update_stats(env: Env, campaign_id: u64, raised: i128, backer_count: u32) {
        let admin = Self::require_admin(&env);
        admin.require_auth();

        if raised < 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        let mut entry = Self::load_entry(&env, campaign_id);
        entry.raised = raised;
        entry.backer_count = backer_count;
        Self::save_entry(&env, &entry);

        events::stats_updated(&env, campaign_id, raised, backer_count);
    }

    /// Closes a gap in the spec: `RegistryStats.total_completed` exists,
    /// but no spec'd function ever increments it. Called by the
    /// admin/orchestrator right after `campaign.mark_completed` succeeds.
    pub fn mark_campaign_completed(env: Env, campaign_id: u64) {
        let admin = Self::require_admin(&env);
        admin.require_auth();

        let mut entry = Self::load_entry(&env, campaign_id);
        if entry.completed {
            panic_with_error!(&env, Error::AlreadyCompleted);
        }
        entry.completed = true;
        Self::save_entry(&env, &entry);

        events::campaign_completed(&env, campaign_id);
    }

    pub fn get_all_campaigns(env: Env) -> Vec<u64> {
        Self::all_ids(&env)
    }

    pub fn get_stats(env: Env) -> RegistryStats {
        let ids = Self::all_ids(&env);
        let mut total_raised: i128 = 0;
        let mut total_backers: u32 = 0;
        let mut total_completed: u32 = 0;

        for id in ids.iter() {
            let entry = Self::load_entry(&env, id);
            total_raised = total_raised
                .checked_add(entry.raised)
                .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidAmount));
            total_backers = total_backers.saturating_add(entry.backer_count);
            if entry.completed {
                total_completed = total_completed.saturating_add(1);
            }
        }

        RegistryStats {
            total_campaigns: ids.len() as u64,
            total_raised,
            total_backers,
            total_completed,
        }
    }

    pub fn get_featured_campaigns(env: Env) -> Vec<u64> {
        Self::featured_ids(&env)
    }

    pub fn set_featured(env: Env, admin: Address, campaign_id: u64, featured: bool) {
        Self::require_admin_auth(&env, &admin);

        // Ensures the campaign is registered before it can be featured.
        Self::load_entry(&env, campaign_id);

        let mut ids = Self::featured_ids(&env);
        let existing_index = ids.iter().position(|id| id == campaign_id);

        if featured {
            if existing_index.is_none() {
                ids.push_back(campaign_id);
            }
        } else if let Some(index) = existing_index {
            ids.remove(index as u32);
        }

        let key = DataKey::FeaturedCampaignIds;
        env.storage().persistent().set(&key, &ids);
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);

        events::featured_set(&env, campaign_id, featured);
    }

    fn require_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
    }

    fn require_admin_auth(env: &Env, admin: &Address) {
        let stored = Self::require_admin(env);
        if *admin != stored {
            panic_with_error!(env, Error::Unauthorized);
        }
        admin.require_auth();
    }

    fn load_entry(env: &Env, campaign_id: u64) -> CampaignEntry {
        env.storage()
            .persistent()
            .get(&DataKey::CampaignEntry(campaign_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::CampaignNotRegistered))
    }

    fn save_entry(env: &Env, entry: &CampaignEntry) {
        let key = DataKey::CampaignEntry(entry.campaign_id);
        env.storage().persistent().set(&key, entry);
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);
    }

    fn all_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::AllCampaignIds)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn featured_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::FeaturedCampaignIds)
            .unwrap_or_else(|| Vec::new(env))
    }
}
