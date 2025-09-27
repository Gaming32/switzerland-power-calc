use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo::rerun-if-changed=splat_lang");

    let mut texts_by_language = phf_codegen::Map::new();
    for language_path in fs::read_dir("splat_lang").unwrap() {
        let language_path = language_path.unwrap();

        let lang_data = fs::read(language_path.path()).unwrap();
        let lang_data: HashMap<&str, HashMap<&str, Cow<'_, str>>> =
            serde_json::from_slice(&lang_data).unwrap();
        let get_translation = |base, sub| lang_data.get(&base).unwrap().get(&sub).unwrap();

        let lang_name = language_path.file_name().to_str().unwrap().strip_suffix(".json").unwrap().to_string();
        texts_by_language.entry(
            lang_name.to_string(),
            phf_codegen::Map::new()
                .entry(
                    "language_name",
                    lit(match lang_name.as_str() {
                        "CNzh" => "中文 (中国)",
                        "EUde" => "Deutsch",
                        "EUen" => "English (United Kingdom)",
                        "EUit" => "italiano",
                        "EUnl" => "Nederlands",
                        "EUru" => "русский",
                        "KRko" => "한국어",
                        "TWzh" => "中文 (台灣)",
                        "USen" => "English (United States)",
                        name => get_translation("CommonMsg/RegionLanguageID", name)
                    })
                )
                .entry(
                    "calculating",
                    lit(match lang_name.as_str() {
                        "USen" | "EUen" => "Calculating Switzerland Power...",
                        "JPja" => "Switzerland パワー 計測中…",
                        _ => get_translation("LayoutMsg/Tml_ListRecord_00", "030"),
                    }),
                )
                .entry(
                    "calculated",
                    lit(get_translation(
                        "LayoutMsg/Lobby_ResultDialogue_00",
                        "T_Rank_00",
                    )),
                )
                .entry(
                    "power_value",
                    lit(&get_translation("CommonMsg/UnitName", "XPower")
                        .replace("[group=0002 type=0000 params=00 04 00 00]", "{integer}")
                        .replace("[group=0002 type=0000 params=01 01 00 00]", "{fraction}")
                    ),
                )
                .entry(
                    "position",
                    lit(get_translation(
                        "LayoutMsg/Lobby_ResultDialogue_00",
                        "T_XRankTitle_00",
                    )),
                )
                .entry(
                    "estimate",
                    lit(get_translation(
                        "LayoutMsg/Lobby_ResultDialogue_00",
                        "T_XRankTitle_01",
                    )),
                )
                .entry(
                    "rank_value",
                    lit(&get_translation("LayoutMsg/Lobby_ResultDialogue_00", "200")
                        .replace("[group=0002 type=0000 params=00 04 00 00]", "{rank}")),
                )
                .build()
                .to_string(),
        );
    }

    let lang_output_path = Path::new(&env::var_os("OUT_DIR").unwrap()).join("splat_lang.rs");
    fs::write(lang_output_path, texts_by_language.build().to_string()).unwrap();
}

fn lit(x: &str) -> String {
    format!("{x:?}")
}
