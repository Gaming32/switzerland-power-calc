use derive_more::with_trait::FromStr;
use std::collections::HashMap;
use strfmt::DisplayStr;

const TEXTS_BY_LANGUAGE: phf::Map<&str, phf::Map<&str, &'static str>> =
    include!(concat!(env!("OUT_DIR"), "/splat_lang.rs"));

pub fn get_text(lang: &str, key: &'static str) -> &'static str {
    TEXTS_BY_LANGUAGE
        .get(lang)
        .or_else(|| TEXTS_BY_LANGUAGE.get("USen"))
        .unwrap()
        .get(key)
        .copied()
        .unwrap_or(key)
}

pub fn format_power(lang: &str, key: &'static str, power: f64) -> String {
    let power = (power * 10.0).floor().abs() as u32;
    let integer = power / 10;
    let fraction = power % 10;
    get_text_fmt(
        lang,
        key,
        [
            (FmtKey::Integer, &full_width_number(integer)),
            (FmtKey::Fraction, &full_width_number(fraction)),
        ],
    )
}

pub fn format_rank(lang: &str, rank: u32) -> String {
    get_text_fmt(
        lang,
        "rank_value",
        [(FmtKey::Rank, &full_width_number(rank))],
    )
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, FromStr)]
enum FmtKey {
    Integer,
    Fraction,
    Rank,
}

fn get_text_fmt<const N: usize>(
    lang: &str,
    key: &'static str,
    values: [(FmtKey, &dyn DisplayStr); N],
) -> String {
    strfmt::strfmt(get_text(lang, key), &HashMap::from(values)).unwrap()
}

/// Encodes a number into fullwidth characters
fn full_width_number(mut x: u32) -> String {
    if x == 0 {
        return "０".to_string();
    }

    const BUF_SIZE: usize = (u32::MAX.ilog10() as usize + 1) * 3;
    let mut result = [0; BUF_SIZE];
    let mut offset = BUF_SIZE;
    while x > 0 {
        let digit = x % 10;
        x /= 10;

        result[offset - 3] = 239;
        result[offset - 2] = 188;
        result[offset - 1] = 144 + digit as u8;
        offset -= 3;
    }

    let result = &result[offset..];
    unsafe { str::from_utf8_unchecked(result) }.to_string()
}
