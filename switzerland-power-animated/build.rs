use hashlink::LinkedHashMap;
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::{env, fs, io};

const DEFAULT_LANGUAGE: &str = "USen";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=splat_lang");

    let mut languages = Vec::new();
    let mut translations = LinkedHashMap::new();
    for language_path in fs::read_dir("splat_lang").unwrap() {
        let language_path = language_path.unwrap();

        let lang_data = fs::read(language_path.path()).unwrap();
        let lang_data: HashMap<&str, HashMap<&str, Cow<'_, str>>> =
            serde_json::from_slice(&lang_data).unwrap();

        let get_translation = |base, sub| lang_data.get(&base).unwrap().get(&sub).unwrap();

        let lang_name = language_path
            .file_name()
            .to_str()
            .unwrap()
            .strip_suffix(".json")
            .unwrap()
            .to_string();

        let mut add_translation = |key, translation_type, value: &str| {
            translations
                .entry(key)
                .or_insert_with(|| (translation_type, LinkedHashMap::new()))
                .1
                .insert(lang_name.clone(), value.to_owned());
        };

        add_translation(
            "language_name",
            TranslationType::Normal,
            match lang_name.as_str() {
                "CNzh" => "中文 (中国)",
                "EUde" => "Deutsch",
                "EUen" => "English (United Kingdom)",
                "EUit" => "italiano",
                "EUnl" => "Nederlands",
                "EUru" => "русский",
                "KRko" => "한국어",
                "TWzh" => "中文 (台灣)",
                "USen" => "English (United States)",
                name => get_translation("CommonMsg/RegionLanguageID", name),
            },
        );
        add_translation(
            "calculating",
            TranslationType::Normal,
            match lang_name.as_str() {
                "USen" | "EUen" => "Calculating Switzerland Power...",
                "JPja" => "スイツァランド パワー 計測中…",
                _ => get_translation("LayoutMsg/Tml_ListRecord_00", "030"),
            },
        );
        add_translation(
            "calculated",
            TranslationType::Normal,
            match lang_name.as_str() {
                "EUnl" => "Switzerland-kracht berekend!",
                _ => get_translation("LayoutMsg/Lobby_ResultDialogue_00", "T_Rank_00"),
            },
        );
        add_translation(
            "power_value",
            TranslationType::Power(true),
            &get_translation("CommonMsg/UnitName", "XPower")
                .replace("[group=0002 type=0000 params=00 04 00 00]", "{integer}")
                .replace("[group=0002 type=0000 params=01 01 00 00]", "{fraction}"),
        );
        add_translation(
            "position",
            TranslationType::Normal,
            get_translation("LayoutMsg/Lobby_ResultDialogue_00", "T_XRankTitle_00"),
        );
        add_translation(
            "estimate",
            TranslationType::Normal,
            get_translation("LayoutMsg/Lobby_ResultDialogue_00", "T_XRankTitle_01"),
        );
        add_translation(
            "rank_value",
            TranslationType::FullWidthNumbers,
            &get_translation("LayoutMsg/Lobby_ResultDialogue_00", "200")
                .replace("[group=0002 type=0000 params=00 04 00 00]", "{rank}"),
        );
        add_translation(
            "power",
            TranslationType::Normal,
            match lang_name.as_str() {
                "USen" | "EUen" => "Switzerland Power",
                "JPja" => "スイツァランド パワー",
                _ => "Switzerland Power",
            },
        );
        add_translation(
            "win",
            TranslationType::Normal,
            get_translation("LayoutMsg/Lobby_ResultClearance_00", "020"),
        );
        add_translation(
            "lose",
            TranslationType::Normal,
            get_translation("LayoutMsg/Lobby_ResultClearance_00", "021"),
        );
        add_translation(
            "power_up",
            TranslationType::Power(false),
            &get_translation("LayoutMsg/Lobby_ResultClearance_00", "210")
                .replace("[group=0002 type=0000 params=00 02 00 00]", "{integer}")
                .replace("[group=0002 type=0000 params=01 01 00 00]", "{fraction}"),
        );
        add_translation(
            "power_down",
            TranslationType::Power(false),
            &get_translation("LayoutMsg/Lobby_ResultClearance_00", "211")
                .replace("[group=0002 type=0000 params=00 02 00 00]", "{integer}")
                .replace("[group=0002 type=0000 params=01 01 00 00]", "{fraction}"),
        );

        languages.push(lang_name);
    }

    let lang_output_path = Path::new(&env::var_os("OUT_DIR").unwrap()).join("splat_lang.rs");
    let mut output = BufWriter::new(File::create(lang_output_path)?);
    writeln!(
        output,
        "#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Hash, derive_more::FromStr)]"
    )?;
    writeln!(output, "pub enum AnimationLanguage {{")?;
    for language in languages {
        if language == DEFAULT_LANGUAGE {
            writeln!(output, "  #[default]")?;
        }
        writeln!(output, "  {language},")?;
    }
    writeln!(output, "}}")?;
    writeln!(output)?;
    writeln!(output, "#[allow(unused_parens, clippy::useless_format)]")?;
    writeln!(output, "impl AnimationLanguage {{")?;
    for (key, (translation_type, translations)) in translations {
        translation_type.generate_function(&mut output, key, translations)?;
        writeln!(output)?;
    }
    writeln!(output, "}}")?;

    Ok(())
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum TranslationType {
    Normal,
    FullWidthNumbers,
    Power(bool),
}

impl TranslationType {
    fn generate_function(
        &self,
        output: &mut impl Write,
        key: &str,
        translations: LinkedHashMap<String, String>,
    ) -> io::Result<()> {
        let arg_names =
            get_format_arg_names(translations.get(&DEFAULT_LANGUAGE.to_string()).unwrap());
        let args_text = match self {
            Self::Normal | Self::FullWidthNumbers => {
                let arg_type = if *self == Self::Normal {
                    "impl Display"
                } else {
                    "u32"
                };
                arg_names
                    .iter()
                    .map(|x| format!(", {x}: {arg_type}"))
                    .collect::<String>()
            }
            Self::Power(_) => ", power: f64".to_string(),
        };
        let formatter = if args_text.is_empty() {
            writeln!(output, "  pub const fn {key}(&self) -> &'static str {{")?;
            ""
        } else {
            writeln!(output, "  pub fn {key}(&self{args_text}) -> String {{")?;
            "format!"
        };
        match self {
            TranslationType::Normal => {}
            TranslationType::FullWidthNumbers => {
                for arg in &arg_names {
                    writeln!(output, "    let {arg} = full_width_number({arg});")?;
                }
            }
            TranslationType::Power(include_negative_sign) => {
                assert_eq!(arg_names, vec!["integer", "fraction"]);
                writeln!(
                    output,
                    "    let (integer, fraction) = format_power_num(power, {include_negative_sign});"
                )?;
            }
        }
        writeln!(output, "    match self {{")?;
        for (language, value) in translations {
            writeln!(output, "      Self::{language} => {formatter}({value:?}),")?;
        }
        writeln!(output, "    }}")?;
        writeln!(output, "  }}")?;
        Ok(())
    }
}

fn get_format_arg_names(message: &str) -> Vec<&str> {
    use parse_format::*;
    Parser::new(message, None, None, false, ParseMode::Format)
        .filter_map(|x| {
            if let NextArgument(Argument {
                position: ArgumentNamed(argument),
                ..
            }) = x
            {
                Some(argument)
            } else {
                None
            }
        })
        .collect()
}
