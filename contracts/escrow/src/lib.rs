#![no_std]

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use types::{
    Contribution, DataKey, EscrowMeta, EscrowState, LEDGER_BUMP, LEDGER_THRESHOLD,
};

use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Env, Vec};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// One-time setup. `admin` is the authority that can register campaigns,
    /// release milestone funds, and enable refunds.
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

    /// Registers a campaign's escrow bucket: which token it accepts and
    /// where milestone releases should be paid out.
    ///
    /// This function is not in the original spec. The spec's `contribute`
    /// and `release_milestone` signatures have no way to name a payout
    /// recipient, and this contract deliberately makes no cross-contract
    /// calls into `campaign` (see docs/escrow.md). `register_campaign` is
    /// the minimal addition that lets the admin/orchestrator mirror the
    /// campaign's creator + token into escrow once, right after
    /// `campaign.create_campaign` succeeds, before any contributions land.
    pub fn register_campaign(env: Env, admin: Address, campaign_id: u64, creator: Address, token: Address) {
        Self::require_admin_auth(&env, &admin);

        if env
            .storage()
            .persistent()
            .has(&DataKey::EscrowMeta(campaign_id))
        {
            panic_with_error!(&env, Error::CampaignAlreadyRegistered);
        }

        let meta = EscrowMeta {
            campaign_id,
            creator: creator.clone(),
            token: token.clone(),
            total_locked: 0,
            total_released: 0,
            total_refunded: 0,
            refundable: false,
        };
        Self::save_meta(&env, &meta);

        events::campaign_registered(&env, campaign_id, &creator, &token);
    }

    /// Marks a campaign's escrow as refundable, unlocking `refund_all` and
    /// `refund_backer`. Not in the original spec — see docs/escrow.md for
    /// why refund eligibility needs an explicit, admin-controlled switch
    /// rather than a live cross-contract read of campaign status.
    pub fn enable_refunds(env: Env, admin: Address, campaign_id: u64) {
        Self::require_admin_auth(&env, &admin);

        let mut meta = Self::load_meta(&env, campaign_id);
        if meta.refundable {
            panic_with_error!(&env, Error::RefundsAlreadyEnabled);
        }
        meta.refundable = true;
        Self::save_meta(&env, &meta);

        events::refunds_enabled(&env, campaign_id);
    }

    pub fn contribute(env: Env, backer: Address, campaign_id: u64, amount: i128, token: Address) {
        backer.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        let mut meta = Self::load_meta(&env, campaign_id);
        if meta.token != token {
            panic_with_error!(&env, Error::TokenMismatch);
        }
        if meta.refundable {
            panic_with_error!(&env, Error::CampaignRefundable);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&backer, &env.current_contract_address(), &amount);

        let key = DataKey::Contribution(campaign_id, backer.clone());
        let existing: Option<Contribution> = env.storage().persistent().get(&key);
        let is_new = existing.is_none();
        let mut contribution = existing.unwrap_or(Contribution {
            backer: backer.clone(),
            campaign_id,
            amount: 0,
            token: token.clone(),
            ledger: env.ledger().sequence(),
            refunded: false,
        });
        contribution.amount = contribution
            .amount
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));

        env.storage().persistent().set(&key, &contribution);
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);

        if is_new {
            let mut backers = Self::campaign_backers(&env, campaign_id);
            backers.push_back(backer.clone());
            let backers_key = DataKey::CampaignBackers(campaign_id);
            env.storage().persistent().set(&backers_key, &backers);
            env.storage()
                .persistent()
                .extend_ttl(&backers_key, LEDGER_THRESHOLD, LEDGER_BUMP);

            let mut campaigns = Self::backer_campaigns(&env, &backer);
            campaigns.push_back(campaign_id);
            let campaigns_key = DataKey::BackerCampaigns(backer.clone());
            env.storage().persistent().set(&campaigns_key, &campaigns);
            env.storage()
                .persistent()
                .extend_ttl(&campaigns_key, LEDGER_THRESHOLD, LEDGER_BUMP);
        }

        meta.total_locked = meta
            .total_locked
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
        Self::save_meta(&env, &meta);

        events::contributed(&env, campaign_id, &backer, amount);
    }

    pub fn release_milestone(
        env: Env,
        admin: Address,
        campaign_id: u64,
        milestone_id: u64,
        amount: i128,
    ) {
        Self::require_admin_auth(&env, &admin);

        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        let mut meta = Self::load_meta(&env, campaign_id);
        let available = meta.total_locked - meta.total_released - meta.total_refunded;
        if amount > available {
            panic_with_error!(&env, Error::InsufficientFunds);
        }

        let token_client = token::Client::new(&env, &meta.token);
        token_client.transfer(&env.current_contract_address(), &meta.creator, &amount);

        meta.total_released = meta
            .total_released
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
        Self::save_meta(&env, &meta);

        events::milestone_released(&env, campaign_id, milestone_id, amount);
    }

    pub fn refund_all(env: Env, campaign_id: u64) {
        let mut meta = Self::load_meta(&env, campaign_id);
        if !meta.refundable {
            panic_with_error!(&env, Error::RefundsNotEnabled);
        }

        let backers = Self::campaign_backers(&env, campaign_id);
        let token_client = token::Client::new(&env, &meta.token);
        let mut refunded_now: i128 = 0;

        for backer in backers.iter() {
            let key = DataKey::Contribution(campaign_id, backer.clone());
            let mut contribution: Contribution = env
                .storage()
                .persistent()
                .get(&key)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ContributionNotFound));

            if contribution.refunded || contribution.amount == 0 {
                continue;
            }

            token_client.transfer(&env.current_contract_address(), &backer, &contribution.amount);
            refunded_now = refunded_now
                .checked_add(contribution.amount)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));

            contribution.refunded = true;
            env.storage().persistent().set(&key, &contribution);
        }

        meta.total_refunded = meta
            .total_refunded
            .checked_add(refunded_now)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
        Self::save_meta(&env, &meta);

        events::refunded_all(&env, campaign_id, refunded_now);
    }

    pub fn refund_backer(env: Env, backer: Address, campaign_id: u64) {
        backer.require_auth();

        let mut meta = Self::load_meta(&env, campaign_id);
        if !meta.refundable {
            panic_with_error!(&env, Error::RefundsNotEnabled);
        }

        let key = DataKey::Contribution(campaign_id, backer.clone());
        let mut contribution: Contribution = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ContributionNotFound));

        if contribution.refunded {
            panic_with_error!(&env, Error::AlreadyRefunded);
        }

        let token_client = token::Client::new(&env, &meta.token);
        token_client.transfer(&env.current_contract_address(), &backer, &contribution.amount);

        contribution.refunded = true;
        env.storage().persistent().set(&key, &contribution);

        meta.total_refunded = meta
            .total_refunded
            .checked_add(contribution.amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
        Self::save_meta(&env, &meta);

        events::refunded_backer(&env, campaign_id, &backer, contribution.amount);
    }

    pub fn get_escrow_state(env: Env, campaign_id: u64) -> EscrowState {
        let meta = Self::load_meta(&env, campaign_id);
        let contributions = Self::assemble_contributions(&env, campaign_id);
        EscrowState {
            campaign_id: meta.campaign_id,
            token: meta.token,
            total_locked: meta.total_locked,
            total_released: meta.total_released,
            total_refunded: meta.total_refunded,
            contributions,
        }
    }

    pub fn get_contribution(env: Env, backer: Address, campaign_id: u64) -> Option<Contribution> {
        env.storage()
            .persistent()
            .get(&DataKey::Contribution(campaign_id, backer))
    }

    pub fn get_contributions_by_campaign(env: Env, campaign_id: u64) -> Vec<Contribution> {
        Self::assemble_contributions(&env, campaign_id)
    }

    pub fn get_contributions_by_backer(env: Env, backer: Address) -> Vec<Contribution> {
        let campaign_ids = Self::backer_campaigns(&env, &backer);
        let mut out = Vec::new(&env);
        for campaign_id in campaign_ids.iter() {
            if let Some(c) = env
                .storage()
                .persistent()
                .get(&DataKey::Contribution(campaign_id, backer.clone()))
            {
                out.push_back(c);
            }
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

    fn load_meta(env: &Env, campaign_id: u64) -> EscrowMeta {
        env.storage()
            .persistent()
            .get(&DataKey::EscrowMeta(campaign_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::CampaignNotRegistered))
    }

    fn save_meta(env: &Env, meta: &EscrowMeta) {
        let key = DataKey::EscrowMeta(meta.campaign_id);
        env.storage().persistent().set(&key, meta);
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);
    }

    fn campaign_backers(env: &Env, campaign_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::CampaignBackers(campaign_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn backer_campaigns(env: &Env, backer: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::BackerCampaigns(backer.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn assemble_contributions(env: &Env, campaign_id: u64) -> Vec<Contribution> {
        let backers = Self::campaign_backers(env, campaign_id);
        let mut out = Vec::new(env);
        for backer in backers.iter() {
            if let Some(c) = env
                .storage()
                .persistent()
                .get(&DataKey::Contribution(campaign_id, backer))
            {
                out.push_back(c);
            }
        }
        out
    }
}
