#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;

fn setup(env: &Env) -> (RegistryContractClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

#[test]
fn test_initialize_sets_admin() {
    let env = Env::default();
    let _ = setup(&env);
}

#[test]
#[should_panic]
fn test_initialize_twice_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    client.initialize(&Address::generate(&env));
}

#[test]
fn test_register_campaign_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);

    let ids = client.get_all_campaigns();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids.get(0).unwrap(), 1u64);

    let stats = client.get_stats();
    assert_eq!(stats.total_campaigns, 1);
    assert_eq!(stats.total_raised, 0);
    assert_eq!(stats.total_backers, 0);
    assert_eq!(stats.total_completed, 0);
}

#[test]
#[should_panic]
fn test_register_campaign_requires_admin_auth() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    env.set_auths(&[]);
    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);
}

#[test]
#[should_panic]
fn test_register_campaign_twice_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);
    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);
}

#[test]
#[should_panic]
fn test_register_campaign_invalid_goal_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    client.register_campaign(&1u64, &creator, &token, &0i128);
}

#[test]
fn test_update_stats_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);

    client.update_stats(&1u64, &500_000i128, &3u32);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 500_000i128);
    assert_eq!(stats.total_backers, 3);
}

#[test]
#[should_panic]
fn test_update_stats_unregistered_campaign_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    client.update_stats(&99u64, &500_000i128, &3u32);
}

#[test]
fn test_mark_campaign_completed_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);

    client.mark_campaign_completed(&1u64);

    let stats = client.get_stats();
    assert_eq!(stats.total_completed, 1);
}

#[test]
#[should_panic]
fn test_mark_campaign_completed_twice_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);

    client.mark_campaign_completed(&1u64);
    client.mark_campaign_completed(&1u64);
}

#[test]
fn test_get_stats_aggregates_across_campaigns() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);
    client.register_campaign(&2u64, &creator, &token, &2_000_000i128);
    client.update_stats(&1u64, &400_000i128, &2u32);
    client.update_stats(&2u64, &900_000i128, &5u32);
    client.mark_campaign_completed(&1u64);

    let stats = client.get_stats();
    assert_eq!(stats.total_campaigns, 2);
    assert_eq!(stats.total_raised, 1_300_000i128);
    assert_eq!(stats.total_backers, 7);
    assert_eq!(stats.total_completed, 1);
}

#[test]
fn test_set_featured_add_and_remove() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);

    assert_eq!(client.get_featured_campaigns().len(), 0);

    client.set_featured(&admin, &1u64, &true);
    let featured = client.get_featured_campaigns();
    assert_eq!(featured.len(), 1);
    assert_eq!(featured.get(0).unwrap(), 1u64);

    // Setting featured=true again should not duplicate the entry.
    client.set_featured(&admin, &1u64, &true);
    assert_eq!(client.get_featured_campaigns().len(), 1);

    client.set_featured(&admin, &1u64, &false);
    assert_eq!(client.get_featured_campaigns().len(), 0);
}

#[test]
#[should_panic]
fn test_set_featured_wrong_admin_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    client.register_campaign(&1u64, &creator, &token, &1_000_000i128);

    let impostor = Address::generate(&env);
    client.set_featured(&impostor, &1u64, &true);
}

#[test]
#[should_panic]
fn test_set_featured_unregistered_campaign_panics() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    client.set_featured(&admin, &99u64, &true);
}

#[test]
fn test_get_all_campaigns_empty_initially() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    assert_eq!(client.get_all_campaigns().len(), 0);
}
