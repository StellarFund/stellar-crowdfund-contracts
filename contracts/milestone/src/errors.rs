use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    MilestonesAlreadyCreated = 4,
    NoMilestonesProvided = 5,
    InvalidAmount = 6,
    InvalidPercentage = 7,
    PercentageSumExceeds100 = 8,
    InvalidDeadline = 9,
    TitleEmpty = 10,
    TitleTooLong = 11,
    DescriptionTooLong = 12,
    ProofUrlTooLong = 13,
    ProofUrlEmpty = 14,
    MilestoneNotFound = 15,
    NotCampaignCreator = 16,
    InvalidMilestoneStatus = 17,
    Overflow = 18,
}
