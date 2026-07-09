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
    InvalidAmount = 6,
    TokenMismatch = 7,
    InsufficientFunds = 8,
    ContributionNotFound = 9,
    AlreadyRefunded = 10,
    RefundsNotEnabled = 11,
    RefundsAlreadyEnabled = 12,
    CampaignRefundable = 13,
    Overflow = 14,
}
