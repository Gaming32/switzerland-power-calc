#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Hash, derive_more::FromStr)]
pub enum AnimationLanguage {
    CNzh,
    EUde,
    EUen,
    EUes,
    EUfr,
    EUit,
    EUnl,
    EUru,
    JPja,
    KRko,
    TWzh,
    #[default]
    USen,
    USes,
    USfr,
}

impl phf_shared::PhfHash for AnimationLanguage {
    fn phf_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hasher::write_u8(state, *self as u8);
    }
}

impl phf_shared::FmtConst for AnimationLanguage {
    fn fmt_const(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("AnimationLanguage::{self:?}"))
    }
}

impl phf_shared::PhfBorrow<AnimationLanguage> for AnimationLanguage {
    fn borrow(&self) -> &AnimationLanguage {
        self
    }
}
