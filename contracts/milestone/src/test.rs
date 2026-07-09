#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::vec;

fn setup(env: &Env) -> (MilestoneContractClient<'_>, Address) {
    env.ledger().with_mut(|li| li.sequence_number = 100);
    env.mock_all_auths();
    let contract_id = env.register(MilestoneContract, ());
    let client = MilestoneContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

fn two_milestones(env: &Env) -> Vec<(String, String, i128, u32, u32)> {
    vec![
        env,
        (
            String::from_str(env, "Prototype"),
            String::from_str(env, "Build a working prototype"),
            400_000i128,
            40u32,
            1_000u32,
        ),
        (
            String::from_str(env, "Launch"),
            String::from_str(env, "Public launch"),
            600_000i128,
            60u32,
            2_000u32,
        ),
    ]
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
fn test_create_milestones_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);

    client.create_milestones(&creator, &1u64, &two_milestones(&env));

    let milestones = client.get_milestones_by_campaign(&1u64);
    assert_eq!(milestones.len(), 2);
    let m0 = milestones.get(0).unwrap();
    assert_eq!(m0.title, String::from_str(&env, "Prototype"));
    assert_eq!(m0.status, MilestoneStatus::Pending);
    assert_eq!(m0.percentage, 40);
}

#[test]
#[should_panic]
fn test_create_milestones_twice_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);

    client.create_milestones(&creator, &1u64, &two_milestones(&env));
    client.create_milestones(&creator, &1u64, &two_milestones(&env));
}

#[test]
#[should_panic]
fn test_create_milestones_empty_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let empty: Vec<(String, String, i128, u32, u32)> = vec![&env];

    client.create_milestones(&creator, &1u64, &empty);
}

#[test]
#[should_panic]
fn test_create_milestones_percentage_over_100_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);

    let milestones = vec![
        &env,
        (
            String::from_str(&env, "A"),
            String::from_str(&env, "desc"),
            500_000i128,
            70u32,
            1_000u32,
        ),
        (
            String::from_str(&env, "B"),
            String::from_str(&env, "desc"),
            500_000i128,
            40u32,
            2_000u32,
        ),
    ];

    client.create_milestones(&creator, &1u64, &milestones);
}

#[test]
#[should_panic]
fn test_create_milestones_zero_amount_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);

    let milestones = vec![
        &env,
        (
            String::from_str(&env, "A"),
            String::from_str(&env, "desc"),
            0i128,
            50u32,
            1_000u32,
        ),
    ];

    client.create_milestones(&creator, &1u64, &milestones);
}

#[test]
fn test_submit_milestone_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));

    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/1"));

    let milestone = client.get_milestone(&1u64, &0u64);
    assert_eq!(milestone.status, MilestoneStatus::UnderReview);
    assert_eq!(milestone.proof_url, String::from_str(&env, "https://proof.example/1"));
}

#[test]
#[should_panic]
fn test_submit_milestone_wrong_creator_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    let impostor = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));

    client.submit_milestone(&impostor, &1u64, &0u64, &String::from_str(&env, "https://proof.example/1"));
}

#[test]
#[should_panic]
fn test_submit_milestone_already_under_review_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));

    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/1"));
    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/2"));
}

#[test]
fn test_approve_milestone_happy_path() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));
    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/1"));

    client.approve_milestone(&admin, &1u64, &0u64);

    let milestone = client.get_milestone(&1u64, &0u64);
    assert_eq!(milestone.status, MilestoneStatus::Approved);
}

#[test]
#[should_panic]
fn test_approve_milestone_wrong_admin_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));
    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/1"));

    let impostor = Address::generate(&env);
    client.approve_milestone(&impostor, &1u64, &0u64);
}

#[test]
#[should_panic]
fn test_approve_milestone_not_under_review_panics() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));

    client.approve_milestone(&admin, &1u64, &0u64);
}

#[test]
fn test_reject_milestone_then_resubmit() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));
    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/1"));

    client.reject_milestone(&admin, &1u64, &0u64);
    let milestone = client.get_milestone(&1u64, &0u64);
    assert_eq!(milestone.status, MilestoneStatus::Rejected);

    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/2"));
    let milestone = client.get_milestone(&1u64, &0u64);
    assert_eq!(milestone.status, MilestoneStatus::UnderReview);
}

#[test]
#[should_panic]
fn test_reject_milestone_not_under_review_panics() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));

    client.reject_milestone(&admin, &1u64, &0u64);
}

#[test]
fn test_mark_released_happy_path() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));
    client.submit_milestone(&creator, &1u64, &0u64, &String::from_str(&env, "https://proof.example/1"));
    client.approve_milestone(&admin, &1u64, &0u64);

    client.mark_released(&admin, &1u64, &0u64);

    let milestone = client.get_milestone(&1u64, &0u64);
    assert_eq!(milestone.status, MilestoneStatus::Released);
    assert!(milestone.completed_at.is_some());
}

#[test]
#[should_panic]
fn test_mark_released_not_approved_panics() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let creator = Address::generate(&env);
    client.create_milestones(&creator, &1u64, &two_milestones(&env));

    client.mark_released(&admin, &1u64, &0u64);
}

#[test]
#[should_panic]
fn test_get_milestone_not_found_panics() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    client.get_milestone(&1u64, &0u64);
}

#[test]
fn test_get_milestones_by_campaign_empty_for_unknown() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let milestones = client.get_milestones_by_campaign(&999u64);
    assert_eq!(milestones.len(), 0);
}
