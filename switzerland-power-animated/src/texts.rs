use derive_more::with_trait::FromStr;
use std::collections::HashMap;

const TEXTS_BY_LANGUAGE: phf::Map<&str, phf::Map<&str, &'static str>> =
    include!(concat!(env!("OUT_DIR"), "/splat_lang.rs"));

pub fn get_text(lang: &str, key: &'static str) -> &'static str {
    TEXTS_BY_LANGUAGE
        .get(lang)
        .or_else(|| TEXTS_BY_LANGUAGE.get("en"))
        .unwrap()
        .get(key)
        .copied()
        .unwrap_or(key)
}

pub fn get_text_fmt<const N: usize>(
    lang: &str,
    key: &'static str,
    values: [(FmtKey, &str); N],
) -> String {
    strfmt::strfmt(get_text(lang, key), &HashMap::from(values)).unwrap()
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, FromStr)]
pub enum FmtKey {
    Rank,
    Integer,
    Fraction,
}

pub fn format_power(lang: &str, key: &'static str, power: f64) -> String {
    let power = (power * 10.0).floor().abs() as u32;
    let integer = power / 10;
    let fraction = power % 10;
    get_text_fmt(
        lang,
        key,
        [
            (FmtKey::Integer, &integer.to_string()),
            (FmtKey::Fraction, &fraction.to_string()),
        ],
    )
}
