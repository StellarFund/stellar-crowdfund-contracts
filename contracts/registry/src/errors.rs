use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    CampaignAlreadyRegistered = 4,
    CampaignNotRegistered = 5,
    InvalidGoal = 6,
    InvalidAmount = 7,
    AlreadyCompleted = 8,
}
