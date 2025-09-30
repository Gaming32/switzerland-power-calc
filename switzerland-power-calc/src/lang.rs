use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use switzerland_power_animated::AnimationLanguage;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Serialize, Deserialize, ValueEnum)]
pub enum Language {
    #[value(name = "zh-CN", alias = "zh")]
    #[serde(rename = "zh-CN")]
    ChineseChina,

    #[value(name = "zh-TW")]
    #[serde(rename = "zh-TW")]
    ChineseTaiwan,

    #[value(name = "nl-NL", alias = "nl")]
    #[serde(rename = "nl-NL")]
    Dutch,

    #[value(name = "en-GB")]
    #[serde(rename = "en-GB")]
    EnglishUnitedKingdom,

    #[value(name = "en-US", alias = "en")]
    #[serde(rename = "en-US")]
    #[default]
    EnglishUnitedStates,

    #[value(name = "fr-CA")]
    #[serde(rename = "fr-CA")]
    FrenchCanada,

    #[value(name = "fr-FR", alias = "fr")]
    #[serde(rename = "fr-FR")]
    FrenchFrance,

    #[value(name = "de-DE", alias = "de")]
    #[serde(rename = "de-DE")]
    German,

    #[value(name = "it-IT", alias = "it")]
    #[serde(rename = "it-IT")]
    Italian,

    #[value(name = "ja-JP", alias = "ja")]
    #[serde(rename = "ja-JP")]
    Japanese,

    #[value(name = "ko-KR", alias = "ko")]
    #[serde(rename = "ko-KR")]
    Korean,

    #[value(name = "ru-RU", alias = "ru")]
    #[serde(rename = "ru-RU")]
    Russian,

    #[value(name = "es-419")]
    #[serde(rename = "es-419")]
    SpanishLatinAmerica,

    #[value(name = "es-ES", alias = "es")]
    #[serde(rename = "es-ES")]
    SpanishSpain,
}

impl From<Language> for AnimationLanguage {
    fn from(val: Language) -> Self {
        match val {
            Language::ChineseChina => AnimationLanguage::CNzh,
            Language::ChineseTaiwan => AnimationLanguage::TWzh,
            Language::Dutch => AnimationLanguage::EUnl,
            Language::EnglishUnitedKingdom => AnimationLanguage::EUen,
            Language::EnglishUnitedStates => AnimationLanguage::USen,
            Language::FrenchCanada => AnimationLanguage::USfr,
            Language::FrenchFrance => AnimationLanguage::EUfr,
            Language::German => AnimationLanguage::EUde,
            Language::Italian => AnimationLanguage::EUit,
            Language::Japanese => AnimationLanguage::JPja,
            Language::Korean => AnimationLanguage::KRko,
            Language::Russian => AnimationLanguage::EUru,
            Language::SpanishLatinAmerica => AnimationLanguage::USes,
            Language::SpanishSpain => AnimationLanguage::EUes,
        }
    }
}
