use soroban_sdk::{symbol_short, Address, Env};

pub fn campaign_created(env: &Env, campaign_id: u64, creator: &Address, goal: i128) {
    env.events()
        .publish((symbol_short!("create"), campaign_id), (creator.clone(), goal));
}

pub fn campaign_cancelled(env: &Env, campaign_id: u64, creator: &Address) {
    env.events()
        .publish((symbol_short!("cancel"), campaign_id), creator.clone());
}

pub fn raised_updated(env: &Env, campaign_id: u64, amount: i128, total_raised: i128) {
    env.events()
        .publish((symbol_short!("raised"), campaign_id), (amount, total_raised));
}

pub fn campaign_funded(env: &Env, campaign_id: u64) {
    env.events()
        .publish((symbol_short!("funded"), campaign_id), ());
}

pub fn campaign_expired(env: &Env, campaign_id: u64) {
    env.events()
        .publish((symbol_short!("expired"), campaign_id), ());
}

pub fn campaign_completed(env: &Env, campaign_id: u64) {
    env.events()
        .publish((symbol_short!("complete"), campaign_id), ());
}
