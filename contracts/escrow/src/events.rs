use soroban_sdk::{symbol_short, Address, Env};

pub fn campaign_registered(env: &Env, campaign_id: u64, creator: &Address, token: &Address) {
    env.events().publish(
        (symbol_short!("register"), campaign_id),
        (creator.clone(), token.clone()),
    );
}

pub fn contributed(env: &Env, campaign_id: u64, backer: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("contrib"), campaign_id),
        (backer.clone(), amount),
    );
}

pub fn milestone_released(env: &Env, campaign_id: u64, milestone_id: u64, amount: i128) {
    env.events().publish(
        (symbol_short!("release"), campaign_id),
        (milestone_id, amount),
    );
}

pub fn refunds_enabled(env: &Env, campaign_id: u64) {
    env.events()
        .publish((symbol_short!("refunden"), campaign_id), ());
}

pub fn refunded_all(env: &Env, campaign_id: u64, total_refunded: i128) {
    env.events()
        .publish((symbol_short!("refundal"), campaign_id), total_refunded);
}

pub fn refunded_backer(env: &Env, campaign_id: u64, backer: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("refundbk"), campaign_id),
        (backer.clone(), amount),
    );
}
