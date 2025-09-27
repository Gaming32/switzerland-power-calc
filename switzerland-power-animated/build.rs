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

        texts_by_language.entry(
            language_path.file_name().to_str().unwrap()[..2].to_string(),
            phf_codegen::Map::new()
                .entry(
                    "calculating",
                    lit(&format!(
                        "Switzerland Power {}", // TODO: Localize Switzerland Power
                        get_translation("LayoutMsg/Tml_ListRecord_00", "030")
                    )),
                )
                .entry(
                    "calculated",
                    lit(get_translation(
                        "LayoutMsg/Lobby_ResultPower_00",
                        "T_Rank_00",
                    )),
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
