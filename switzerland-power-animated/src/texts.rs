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
