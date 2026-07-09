use soroban_sdk::{symbol_short, Address, Env};

pub fn campaign_registered(env: &Env, campaign_id: u64, creator: &Address, goal: i128) {
    env.events().publish(
        (symbol_short!("register"), campaign_id),
        (creator.clone(), goal),
    );
}

pub fn stats_updated(env: &Env, campaign_id: u64, raised: i128, backer_count: u32) {
    env.events().publish(
        (symbol_short!("stats"), campaign_id),
        (raised, backer_count),
    );
}

pub fn campaign_completed(env: &Env, campaign_id: u64) {
    env.events()
        .publish((symbol_short!("complete"), campaign_id), ());
}

pub fn featured_set(env: &Env, campaign_id: u64, featured: bool) {
    env.events()
        .publish((symbol_short!("featured"), campaign_id), featured);
}
