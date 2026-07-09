#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};

fn setup(env: &Env) -> (CampaignContractClient<'_>, Address) {
    env.ledger().with_mut(|li| li.sequence_number = 100);
    let contract_id = env.register(CampaignContract, ());
    let client = CampaignContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn create_test_campaign(env: &Env, client: &CampaignContractClient, creator: &Address) -> u64 {
    let token = Address::generate(env);
    client.create_campaign(
        creator,
        &String::from_str(env, "Save the Reef"),
        &String::from_str(env, "A campaign to restore coral reefs"),
        &token,
        &1_000_000i128,
        &1_000u32,
        &3u32,
        &String::from_str(env, "https://example.com"),
        &String::from_str(env, "https://example.com/img.png"),
    )
}

#[test]
fn test_initialize_sets_admin() {
    let env = Env::default();
    let (_client, _admin) = setup(&env);
}

#[test]
#[should_panic]
fn test_initialize_twice_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let another_admin = Address::generate(&env);
    client.initialize(&another_admin);
}

#[test]
fn test_create_campaign_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);

    let id = create_test_campaign(&env, &client, &creator);
    assert_eq!(id, 0);

    let campaign = client.get_campaign(&id);
    assert_eq!(campaign.creator, creator);
    assert_eq!(campaign.goal, 1_000_000i128);
    assert_eq!(campaign.raised, 0);
    assert_eq!(campaign.backer_count, 0);
    assert_eq!(campaign.status, CampaignStatus::Active);
    assert_eq!(campaign.milestone_count, 3);
}

#[test]
#[should_panic]
fn test_create_campaign_requires_creator_auth() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    // Reset auth mocking so require_auth has nothing to satisfy it with.
    env.set_auths(&[]);
    client.create_campaign(
        &creator,
        &String::from_str(&env, "T"),
        &String::from_str(&env, "D"),
        &token,
        &1_000i128,
        &1_000u32,
        &1u32,
        &String::from_str(&env, ""),
        &String::from_str(&env, ""),
    );
}

#[test]
#[should_panic]
fn test_create_campaign_invalid_goal_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.create_campaign(
        &creator,
        &String::from_str(&env, "T"),
        &String::from_str(&env, "D"),
        &token,
        &0i128,
        &1_000u32,
        &1u32,
        &String::from_str(&env, ""),
        &String::from_str(&env, ""),
    );
}

#[test]
#[should_panic]
fn test_create_campaign_invalid_deadline_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.create_campaign(
        &creator,
        &String::from_str(&env, "T"),
        &String::from_str(&env, "D"),
        &token,
        &1_000i128,
        &10u32, // <= current ledger sequence (100)
        &1u32,
        &String::from_str(&env, ""),
        &String::from_str(&env, ""),
    );
}

#[test]
#[should_panic]
fn test_create_campaign_zero_milestone_count_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.create_campaign(
        &creator,
        &String::from_str(&env, "T"),
        &String::from_str(&env, "D"),
        &token,
        &1_000i128,
        &1_000u32,
        &0u32,
        &String::from_str(&env, ""),
        &String::from_str(&env, ""),
    );
}

#[test]
fn test_cancel_campaign_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.cancel_campaign(&creator, &id);

    let campaign = client.get_campaign(&id);
    assert_eq!(campaign.status, CampaignStatus::Cancelled);
}

#[test]
#[should_panic]
fn test_cancel_campaign_wrong_creator_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    let impostor = Address::generate(&env);
    client.cancel_campaign(&impostor, &id);
}

#[test]
#[should_panic]
fn test_cancel_campaign_twice_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.cancel_campaign(&creator, &id);
    client.cancel_campaign(&creator, &id);
}

#[test]
#[should_panic]
fn test_get_campaign_not_found_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    client.get_campaign(&999u64);
}

#[test]
fn test_get_campaigns_by_creator() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let other = Address::generate(&env);

    create_test_campaign(&env, &client, &creator);
    create_test_campaign(&env, &client, &creator);
    create_test_campaign(&env, &client, &other);

    let mine = client.get_campaigns_by_creator(&creator);
    assert_eq!(mine.len(), 2);

    let theirs = client.get_campaigns_by_creator(&other);
    assert_eq!(theirs.len(), 1);
}

#[test]
fn test_get_active_campaigns_filters_status() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);

    let id1 = create_test_campaign(&env, &client, &creator);
    let id2 = create_test_campaign(&env, &client, &creator);

    client.cancel_campaign(&creator, &id1);

    let active = client.get_active_campaigns();
    assert_eq!(active.len(), 1);
    assert_eq!(active.get(0).unwrap().id, id2);
}

#[test]
fn test_update_raised_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.update_raised(&id, &500_000i128);
    client.update_raised(&id, &250_000i128);

    let campaign = client.get_campaign(&id);
    assert_eq!(campaign.raised, 750_000i128);
    assert_eq!(campaign.backer_count, 2);
}

#[test]
#[should_panic]
fn test_update_raised_zero_amount_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.update_raised(&id, &0i128);
}

#[test]
#[should_panic]
fn test_update_raised_on_inactive_campaign_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.cancel_campaign(&creator, &id);
    client.update_raised(&id, &1_000i128);
}

#[test]
#[should_panic]
fn test_update_raised_without_initialize_panics() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.sequence_number = 100);
    let contract_id = env.register(CampaignContract, ());
    let client = CampaignContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    // No initialize() call, so there is no admin configured.
    client.update_raised(&0u64, &1_000i128);
}

#[test]
fn test_mark_funded_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.update_raised(&id, &1_000_000i128);
    client.mark_funded(&id);

    let campaign = client.get_campaign(&id);
    assert_eq!(campaign.status, CampaignStatus::Funded);
}

#[test]
#[should_panic]
fn test_mark_funded_goal_not_reached_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.update_raised(&id, &1_000i128);
    client.mark_funded(&id);
}

#[test]
fn test_mark_expired_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    env.ledger().with_mut(|li| li.sequence_number = 2_000);
    client.mark_expired(&id);

    let campaign = client.get_campaign(&id);
    assert_eq!(campaign.status, CampaignStatus::Expired);
}

#[test]
#[should_panic]
fn test_mark_expired_before_deadline_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.mark_expired(&id);
}

#[test]
fn test_mark_completed_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.update_raised(&id, &1_000_000i128);
    client.mark_funded(&id);
    client.mark_completed(&id);

    let campaign = client.get_campaign(&id);
    assert_eq!(campaign.status, CampaignStatus::Completed);
}

#[test]
#[should_panic]
fn test_mark_completed_not_funded_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let id = create_test_campaign(&env, &client, &creator);

    client.mark_completed(&id);
}
