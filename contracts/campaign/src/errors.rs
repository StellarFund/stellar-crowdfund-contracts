use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    CampaignNotFound = 3,
    NotCampaignCreator = 4,
    InvalidGoal = 5,
    InvalidDeadline = 6,
    InvalidMilestoneCount = 7,
    CampaignNotActive = 8,
    CampaignNotFunded = 9,
    InvalidAmount = 10,
    TitleTooLong = 11,
    TitleEmpty = 12,
    DescriptionTooLong = 13,
    UrlTooLong = 14,
    DeadlineNotReached = 15,
    GoalNotReached = 16,
    Overflow = 17,
}
