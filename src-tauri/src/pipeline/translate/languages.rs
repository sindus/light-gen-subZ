/// Curated list of languages exposed in the UI, mapping a short code (matching what
/// whisper.cpp reports, and used as the DeepL source/target code where compatible)
/// to its NLLB/FLORES-200 code (used by the local translation model) and display name.
#[derive(serde::Serialize)]
pub struct Language {
    pub code: &'static str,
    pub flores_code: &'static str,
    pub name: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        flores_code: "eng_Latn",
        name: "English",
    },
    Language {
        code: "fr",
        flores_code: "fra_Latn",
        name: "French",
    },
    Language {
        code: "es",
        flores_code: "spa_Latn",
        name: "Spanish",
    },
    Language {
        code: "de",
        flores_code: "deu_Latn",
        name: "German",
    },
    Language {
        code: "it",
        flores_code: "ita_Latn",
        name: "Italian",
    },
    Language {
        code: "pt",
        flores_code: "por_Latn",
        name: "Portuguese",
    },
    Language {
        code: "nl",
        flores_code: "nld_Latn",
        name: "Dutch",
    },
    Language {
        code: "ru",
        flores_code: "rus_Cyrl",
        name: "Russian",
    },
    Language {
        code: "ja",
        flores_code: "jpn_Jpan",
        name: "Japanese",
    },
    Language {
        code: "zh",
        flores_code: "zho_Hans",
        name: "Chinese (Simplified)",
    },
    Language {
        code: "ko",
        flores_code: "kor_Hang",
        name: "Korean",
    },
    Language {
        code: "ar",
        flores_code: "arb_Arab",
        name: "Arabic",
    },
    Language {
        code: "tr",
        flores_code: "tur_Latn",
        name: "Turkish",
    },
    Language {
        code: "pl",
        flores_code: "pol_Latn",
        name: "Polish",
    },
    Language {
        code: "vi",
        flores_code: "vie_Latn",
        name: "Vietnamese",
    },
    Language {
        code: "sv",
        flores_code: "swe_Latn",
        name: "Swedish",
    },
    Language {
        code: "hi",
        flores_code: "hin_Deva",
        name: "Hindi",
    },
    Language {
        code: "uk",
        flores_code: "ukr_Cyrl",
        name: "Ukrainian",
    },
    Language {
        code: "el",
        flores_code: "ell_Grek",
        name: "Greek",
    },
    Language {
        code: "cs",
        flores_code: "ces_Latn",
        name: "Czech",
    },
    Language {
        code: "ro",
        flores_code: "ron_Latn",
        name: "Romanian",
    },
    Language {
        code: "da",
        flores_code: "dan_Latn",
        name: "Danish",
    },
    Language {
        code: "fi",
        flores_code: "fin_Latn",
        name: "Finnish",
    },
    Language {
        code: "id",
        flores_code: "ind_Latn",
        name: "Indonesian",
    },
    Language {
        code: "th",
        flores_code: "tha_Thai",
        name: "Thai",
    },
    Language {
        code: "he",
        flores_code: "heb_Hebr",
        name: "Hebrew",
    },
];

pub fn flores_code_for(short_code: &str) -> Option<&'static str> {
    LANGUAGES
        .iter()
        .find(|l| l.code.eq_ignore_ascii_case(short_code))
        .map(|l| l.flores_code)
}
