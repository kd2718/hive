use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeError {
    #[error("{found} is not a valid ChallengeVisibility")]
    InvalidChallengeVisibility { found: String },
    #[error("Couldn't find challenge creator (uid {0})")]
    MissingChallenger(String),
    #[error("You can't accept your own challenges!")]
    OwnChallenge,
    #[error("This is not your challenge")]
    NotUserChallenge,
    #[error("{found} is not a valid TimeMode")]
    NotValidTimeMode { found: String },
    #[error("Your rating {rating} is outside the rating band {band_lower}-{band_upper}")]
    OutsideBand {
        rating: u64,
        band_upper: u64,
        band_lower: u64,
    },
}
