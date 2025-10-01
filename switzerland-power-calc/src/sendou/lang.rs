use clap::ValueEnum;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serenity::all::{CommandId, Mention};
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use switzerland_power_animated::AnimationLanguage;

#[derive(Copy, Clone, Eq, PartialEq, Default, Serialize, Deserialize, ValueEnum)]
pub enum Language {
    #[value(name = "zh-CN")]
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

// Loosely based on https://support.apple.com/guide/apple-business-connect/language-list-for-showcases-abcb88b4fea6/web because why not
impl Language {
    pub fn supported_languages() -> &'static [Self] {
        Self::value_variants()
    }

    pub fn guess_from_country(country: &str) -> Option<Self> {
        Some(match country {
            "CN" => Language::ChineseChina,
            "TW" => Language::ChineseTaiwan,
            "NL" | "SR" => Language::Dutch,
            "GB" => Language::EnglishUnitedKingdom,
            // We have no en-CA, so we'll have to do with en-US for Canada (it's what Nintendo uses for Canada)
            "US" | "CA" => Language::EnglishUnitedStates,
            "FR" | "GF" => Language::FrenchFrance,
            "DE" => Language::German,
            "IT" => Language::Italian,
            "JP" => Language::Japanese,
            "KR" => Language::Korean,
            "RU" => Language::Russian,
            "AR" | "BO" | "CL" | "CO" | "CR" | "EC" | "SV" | "GT" | "HN" | "MX" | "NI" | "PA"
            | "PY" | "PE" | "UY" | "VE" => Language::SpanishLatinAmerica,
            "ES" => Language::SpanishSpain,
            _ => return None,
        })
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::ChineseChina => "zh-CN",
            Self::ChineseTaiwan => "zh-TW",
            Self::Dutch => "nl-NL",
            Self::EnglishUnitedKingdom => "en-GB",
            Self::EnglishUnitedStates => "en-US",
            Self::FrenchCanada => "fr-CA",
            Self::FrenchFrance => "fr-FR",
            Self::German => "de-DE",
            Self::Italian => "it-IT",
            Self::Japanese => "ja-JP",
            Self::Korean => "ko-KR",
            Self::Russian => "ru-RU",
            Self::SpanishLatinAmerica => "es-419",
            Self::SpanishSpain => "es-ES",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "zh-CN" => Self::ChineseChina,
            "zh-TW" => Self::ChineseTaiwan,
            "nl-NL" => Self::Dutch,
            "en-GB" => Self::EnglishUnitedKingdom,
            "en-US" => Self::EnglishUnitedStates,
            "fr-CA" => Self::FrenchCanada,
            "fr-FR" => Self::FrenchFrance,
            "de-DE" => Self::German,
            "it-IT" => Self::Italian,
            "ja-JP" => Self::Japanese,
            "ko-KR" => Self::Korean,
            "ru-RU" => Self::Russian,
            "es-419" => Self::SpanishLatinAmerica,
            "es-ES" => Self::SpanishSpain,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        AnimationLanguage::from(*self).language_name()
    }

    pub fn discord_id(&self) -> Option<&'static str> {
        match self {
            Self::ChineseChina => Some("zh-CN"),
            Self::ChineseTaiwan => Some("zh-TW"),
            Self::Dutch => Some("nl"),
            Self::EnglishUnitedKingdom => Some("en-GB"),
            Self::EnglishUnitedStates => Some("en-US"),
            Self::FrenchCanada => None,
            Self::FrenchFrance => Some("fr"),
            Self::German => Some("de"),
            Self::Italian => Some("it"),
            Self::Japanese => Some("ja"),
            Self::Korean => Some("ko"),
            Self::Russian => Some("ru"),
            Self::SpanishLatinAmerica => Some("es-419"),
            Self::SpanishSpain => Some("es-ES"),
        }
    }

    pub fn from_discord_id(id: &str) -> Option<Self> {
        Some(match id {
            "de" => Self::German,
            "en-GB" => Self::EnglishUnitedKingdom,
            "en-US" => Self::EnglishUnitedStates,
            "es-ES" => Self::SpanishSpain,
            "es-419" => Self::SpanishLatinAmerica,
            "fr" => Self::FrenchFrance,
            "it" => Self::Italian,
            "nl" => Self::Dutch,
            "ru" => Self::Russian,
            "zh-CN" => Self::ChineseChina,
            "ja" => Self::Japanese,
            "zh-TW" => Self::ChineseTaiwan,
            "ko" => Self::Korean,
            _ => return None,
        })
    }

    /// Returns the specific for this language, if any
    pub fn specific_fallback(&self) -> Option<Self> {
        match self {
            Self::EnglishUnitedKingdom => Some(Self::EnglishUnitedStates),
            Self::FrenchCanada => Some(Self::FrenchFrance),
            Self::SpanishLatinAmerica => Some(Self::SpanishSpain),
            _ => None,
        }
    }

    pub fn fallback(&self) -> Option<Self> {
        match self.specific_fallback() {
            f @ Some(_) => f,
            None if *self == Self::default() => None,
            None => Some(Self::default()),
        }
    }
}

impl Debug for Language {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Display)]
#[display("</{_0}:{_1}>")]
pub struct CommandIdDisplay(pub Cow<'static, str>, pub CommandId);

macro_rules! language_messages {
    {$(
        $message_id:ident$(($($arg_name:ident: $arg_type:ty),+))? => {
            $($language:pat => $message:literal,)+
        },
    )*} => {
        impl Language {
            $(
                #[allow(unreachable_patterns)]
                pub fn $message_id(&self$(, $($arg_name: $arg_type),+)?) -> Cow<'static, str> {
                    let localize = |lang| language_messages!(@message lang, ($(($($arg_name),+))?), $($language => $message,)+);
                    localize(*self)
                        .or_else(|| self.specific_fallback().and_then(localize))
                        .or_else(|| localize(Self::default()))
                        .unwrap_or(Cow::Borrowed(stringify!($message_id)))
                }
            )*
        }
    };

    (@message $language_var:ident, (), $($language:pat => $message:literal,)+) => {
        match $language_var {
            $($language => Some(Cow::Borrowed($message)),)+
            _ => None,
        }
    };

    (@message $language_var:ident, (($($_:ident),+)), $($language:pat => $message:literal,)+) => {
        match $language_var {
            $($language => Some(Cow::Owned(format!($message))),)+
            _ => None,
        }
    };
}

language_messages! {
    bot_crashed(language_command: &CommandIdDisplay) => {
        Language::EnglishUnitedStates => "(the bot crashed and needed a restart; you may see some duplicated messages below; you may also have to set your {language_command} again)",
    },
    channel_explanation(user: Mention) => {
        Language::EnglishUnitedStates => "{user} in this channel, you will receive live updates for your Switzerland Power throughout the tournament.",
    },
    language_command_name => {
        Language::EnglishUnitedStates => "language",
        Language::ChineseChina => "语言",
        Language::ChineseTaiwan => "語言",
        Language::Dutch => "taal",
        Language::FrenchFrance => "langue",
        Language::German => "sprache",
        Language::Italian => "lingua",
        Language::Japanese => "言語",
        Language::Korean => "언어",
        Language::Russian => "язык",
        Language::SpanishSpain => "lengua",
    },
    language_command_desc => {
        Language::EnglishUnitedStates => "Changes the language this bot's messages will be shown to you in",
    },
    language_command_arg_desc => {
        Language::EnglishUnitedStates => "The language to use. If not specified, the language of your Discord client is used",
    },
    language_command_explanation(command: &CommandIdDisplay) => {
        Language::EnglishUnitedStates => "You can use the {command} command to change languages for these messages. (Translations are best effort and some or most may be missing.)",
    },
    changed_language(language: Language) => {
        Language::EnglishUnitedStates => "Bot language changed to {language}",
    },
}
