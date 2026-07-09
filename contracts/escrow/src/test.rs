#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};

fn create_token_contract<'a>(
    env: &'a Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(env, &sac.address()),
        token::StellarAssetClient::new(env, &sac.address()),
    )
}

struct Setup<'a> {
    client: EscrowContractClient<'a>,
    admin: Address,
    token_address: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
}

fn setup(env: &Env) -> Setup<'_> {
    env.ledger().with_mut(|li| li.sequence_number = 100);
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_issuer = Address::generate(env);
    let (token, token_admin) = create_token_contract(env, &token_issuer);

    Setup {
        client,
        admin,
        token_address: token.address.clone(),
        token,
        token_admin,
    }
}

fn register_campaign(s: &Setup, campaign_id: u64, creator: &Address) {
    s.client
        .register_campaign(&s.admin, &campaign_id, creator, &s.token_address);
}

#[test]
fn test_initialize_sets_admin() {
    let env = Env::default();
    let _s = setup(&env);
}

#[test]
#[should_panic]
fn test_initialize_twice_panics() {
    let env = Env::default();
    let s = setup(&env);
    let another_admin = Address::generate(&env);
    s.client.initialize(&another_admin);
}

#[test]
fn test_register_campaign_happy_path() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);

    register_campaign(&s, 1, &creator);

    let state = s.client.get_escrow_state(&1u64);
    assert_eq!(state.campaign_id, 1);
    assert_eq!(state.token, s.token_address);
    assert_eq!(state.total_locked, 0);
    assert_eq!(state.contributions.len(), 0);
}

#[test]
#[should_panic]
fn test_register_campaign_wrong_admin_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    let impostor = Address::generate(&env);

    s.client
        .register_campaign(&impostor, &1u64, &creator, &s.token_address);
}

#[test]
#[should_panic]
fn test_register_campaign_twice_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);

    register_campaign(&s, 1, &creator);
    register_campaign(&s, 1, &creator);
}

#[test]
fn test_contribute_happy_path_accumulates() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &1_000i128);

    s.client.contribute(&backer, &1u64, &400i128, &s.token_address);
    s.client.contribute(&backer, &1u64, &600i128, &s.token_address);

    let contribution = s.client.get_contribution(&backer, &1u64).unwrap();
    assert_eq!(contribution.amount, 1_000i128);
    assert_eq!(s.token.balance(&backer), 0);
    assert_eq!(s.token.balance(&s.client.address), 1_000i128);

    let state = s.client.get_escrow_state(&1u64);
    assert_eq!(state.total_locked, 1_000i128);
    assert_eq!(state.contributions.len(), 1);
}

#[test]
#[should_panic]
fn test_contribute_unregistered_campaign_panics() {
    let env = Env::default();
    let s = setup(&env);
    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &1_000i128);

    s.client.contribute(&backer, &99u64, &100i128, &s.token_address);
}

#[test]
#[should_panic]
fn test_contribute_token_mismatch_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    let other_issuer = Address::generate(&env);
    let (_other_token, other_token_admin) = create_token_contract(&env, &other_issuer);
    let wrong_token = Address::generate(&env);
    let _ = other_token_admin;

    s.client.contribute(&backer, &1u64, &100i128, &wrong_token);
}

#[test]
#[should_panic]
fn test_contribute_zero_amount_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.client.contribute(&backer, &1u64, &0i128, &s.token_address);
}

#[test]
fn test_release_milestone_happy_path() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &1_000i128);
    s.client.contribute(&backer, &1u64, &1_000i128, &s.token_address);

    s.client.release_milestone(&s.admin, &1u64, &0u64, &400i128);

    assert_eq!(s.token.balance(&creator), 400i128);
    let state = s.client.get_escrow_state(&1u64);
    assert_eq!(state.total_released, 400i128);
}

#[test]
#[should_panic]
fn test_release_milestone_insufficient_funds_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &1_000i128);
    s.client.contribute(&backer, &1u64, &1_000i128, &s.token_address);

    s.client.release_milestone(&s.admin, &1u64, &0u64, &2_000i128);
}

#[test]
#[should_panic]
fn test_release_milestone_wrong_admin_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &1_000i128);
    s.client.contribute(&backer, &1u64, &1_000i128, &s.token_address);

    let impostor = Address::generate(&env);
    s.client.release_milestone(&impostor, &1u64, &0u64, &100i128);
}

#[test]
fn test_enable_refunds_happy_path() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    s.client.enable_refunds(&s.admin, &1u64);
}

#[test]
#[should_panic]
fn test_enable_refunds_twice_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    s.client.enable_refunds(&s.admin, &1u64);
    s.client.enable_refunds(&s.admin, &1u64);
}

#[test]
fn test_refund_all_happy_path() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer1 = Address::generate(&env);
    let backer2 = Address::generate(&env);
    s.token_admin.mint(&backer1, &300i128);
    s.token_admin.mint(&backer2, &700i128);
    s.client.contribute(&backer1, &1u64, &300i128, &s.token_address);
    s.client.contribute(&backer2, &1u64, &700i128, &s.token_address);

    s.client.enable_refunds(&s.admin, &1u64);
    s.client.refund_all(&1u64);

    assert_eq!(s.token.balance(&backer1), 300i128);
    assert_eq!(s.token.balance(&backer2), 700i128);
    let state = s.client.get_escrow_state(&1u64);
    assert_eq!(state.total_refunded, 1_000i128);
    assert!(s.client.get_contribution(&backer1, &1u64).unwrap().refunded);
    assert!(s.client.get_contribution(&backer2, &1u64).unwrap().refunded);
}

#[test]
fn test_refund_backer_happy_path() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &500i128);
    s.client.contribute(&backer, &1u64, &500i128, &s.token_address);

    s.client.enable_refunds(&s.admin, &1u64);
    s.client.refund_backer(&backer, &1u64);

    assert_eq!(s.token.balance(&backer), 500i128);
    assert!(s.client.get_contribution(&backer, &1u64).unwrap().refunded);
}

#[test]
#[should_panic]
fn test_refund_backer_before_enabled_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &500i128);
    s.client.contribute(&backer, &1u64, &500i128, &s.token_address);

    s.client.refund_backer(&backer, &1u64);
}

#[test]
#[should_panic]
fn test_refund_backer_already_refunded_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &500i128);
    s.client.contribute(&backer, &1u64, &500i128, &s.token_address);

    s.client.enable_refunds(&s.admin, &1u64);
    s.client.refund_backer(&backer, &1u64);
    s.client.refund_backer(&backer, &1u64);
}

#[test]
#[should_panic]
fn test_contribute_after_refundable_panics() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);
    s.client.enable_refunds(&s.admin, &1u64);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &500i128);
    s.client.contribute(&backer, &1u64, &500i128, &s.token_address);
}

#[test]
fn test_get_contribution_returns_none_for_unknown_backer() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);

    let stranger = Address::generate(&env);
    assert_eq!(s.client.get_contribution(&stranger, &1u64), None);
}

#[test]
fn test_get_contributions_by_campaign_and_backer() {
    let env = Env::default();
    let s = setup(&env);
    let creator = Address::generate(&env);
    register_campaign(&s, 1, &creator);
    register_campaign(&s, 2, &creator);

    let backer = Address::generate(&env);
    s.token_admin.mint(&backer, &1_000i128);
    s.client.contribute(&backer, &1u64, &200i128, &s.token_address);
    s.client.contribute(&backer, &2u64, &300i128, &s.token_address);

    let by_campaign = s.client.get_contributions_by_campaign(&1u64);
    assert_eq!(by_campaign.len(), 1);
    assert_eq!(by_campaign.get(0).unwrap().amount, 200i128);

    let by_backer = s.client.get_contributions_by_backer(&backer);
    assert_eq!(by_backer.len(), 2);
}
