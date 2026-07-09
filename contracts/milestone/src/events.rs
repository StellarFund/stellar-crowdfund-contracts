use soroban_sdk::{symbol_short, Env};

pub fn milestones_created(env: &Env, campaign_id: u64, count: u32) {
    env.events()
        .publish((symbol_short!("created"), campaign_id), count);
}

pub fn milestone_submitted(env: &Env, campaign_id: u64, milestone_id: u64) {
    env.events()
        .publish((symbol_short!("submit"), campaign_id), milestone_id);
}

pub fn milestone_approved(env: &Env, campaign_id: u64, milestone_id: u64) {
    env.events()
        .publish((symbol_short!("approve"), campaign_id), milestone_id);
}

pub fn milestone_rejected(env: &Env, campaign_id: u64, milestone_id: u64) {
    env.events()
        .publish((symbol_short!("reject"), campaign_id), milestone_id);
}

pub fn milestone_released(env: &Env, campaign_id: u64, milestone_id: u64) {
    env.events()
        .publish((symbol_short!("released"), campaign_id), milestone_id);
}
