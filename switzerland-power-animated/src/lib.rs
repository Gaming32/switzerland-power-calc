mod alignment;
mod animation;
mod async_generator;
mod error;
mod font;
mod generator;
mod layout;
mod panes;
mod status;
mod texts;

pub use async_generator::AsyncAnimationGenerator;
pub use error::{Error, Result};
pub use generator::AnimationGenerator;
pub use status::{MatchOutcome, PowerStatus};
pub use texts::AnimationLanguage;
