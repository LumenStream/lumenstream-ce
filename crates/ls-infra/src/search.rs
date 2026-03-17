use std::sync::OnceLock;

use pinyin::ToPinyin;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchKeys {
    pub text: String,
    pub pinyin: String,
    pub initials: String,
}

impl SearchKeys {}

pub fn normalize_media_title(input: &str) -> String {
    let raw = input.trim();
    let mut out = extension_re().replace(raw, "").into_owned();
    out = out.replace(['_', '.'], " ");
    out = bracket_year_re().replace_all(&out, " ").into_owned();
    out = resolution_re().replace_all(&out, " ").into_owned();
    out = release_group_re().replace_all(&out, " ").into_owned();
    out = separator_block_re().replace_all(&out, " ").into_owned();
    out = multi_space_re().replace_all(&out, " ").into_owned();
    out = remove_standalone_year_tokens(&out);
    out = out
        .trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '-' | '_' | '.' | '|' | '/'))
        .to_string();

    if year_token_re().is_match(out.trim())
        && (resolution_re().is_match(raw)
            || bracket_year_re().is_match(raw)
            || raw.contains('-')
            || raw.contains('_')
            || raw.contains('.')
            || raw.contains(' '))
    {
        out.clear();
    }

    out
}

pub fn build_search_keys(input: &str) -> SearchKeys {
    let text = normalize_text(input);
    if text.is_empty() {
        return SearchKeys::default();
    }

    let mut pinyin = String::new();
    let mut initials = String::new();

    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }

        if let Some(py) = ch.to_pinyin() {
            let plain = py.plain();
            pinyin.push_str(plain);
            if let Some(initial) = plain.chars().next() {
                initials.push(initial);
            }
            continue;
        }

        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                if lower.is_alphanumeric() {
                    pinyin.push(lower);
                    initials.push(lower);
                }
            }
        }
    }

    if pinyin.is_empty() {
        pinyin = text
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
    }

    if initials.is_empty() {
        initials = pinyin.clone();
    }

    SearchKeys {
        text,
        pinyin,
        initials,
    }
}

fn normalize_text(input: &str) -> String {
    let mut out = String::new();
    let mut last_space = true;

    for ch in input.nfkc().flat_map(char::to_lowercase) {
        if is_search_char(ch) {
            out.push(ch);
            last_space = false;
            continue;
        }

        if !last_space {
            out.push(' ');
            last_space = true;
        }
    }

    out.trim().to_string()
}

fn is_search_char(ch: char) -> bool {
    ch.is_alphanumeric() || is_cjk(ch)
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
    )
}

fn extension_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\.(?:mp4|mkv|avi|mov|wmv|flv|webm|m4v|ts|iso|vob|m2ts|srt|ass|ssa|vtt|sub|idx)$",
        )
        .expect("compile extension regex")
    })
}

fn release_group_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)[-\s_]*\[(?:zmz|subhd|yyets|zimuzu|chs|cht|eng)\]?[-\s_]*")
            .expect("compile release group regex")
    })
}

fn bracket_year_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)[\(\[\{（【]\s*(?:19|20)\d{2}\s*[\)\]\}）】]")
            .expect("compile bracket-year regex")
    })
}

fn resolution_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(?:\d{3,4}p|[248]k|\d{3,4}x\d{3,4})\b")
            .expect("compile resolution regex")
    })
}

fn separator_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+[-_.|/]+\s+").expect("compile separator-block regex"))
}

fn multi_space_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").expect("compile multi-space regex"))
}

fn year_token_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(?:19|20)\d{2}$").expect("compile year-token regex"))
}

fn remove_standalone_year_tokens(input: &str) -> String {
    let tokens = input
        .split_whitespace()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    if tokens.len() <= 1 {
        return tokens.join(" ");
    }

    let filtered = tokens
        .into_iter()
        .filter(|token| !year_token_re().is_match(token))
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        input.trim().to_string()
    } else {
        filtered.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::{SearchKeys, build_search_keys, normalize_media_title};

    #[test]
    fn build_search_keys_for_chinese_title() {
        let keys = build_search_keys("中文");
        assert_eq!(
            keys,
            SearchKeys {
                text: "中文".to_string(),
                pinyin: "zhongwen".to_string(),
                initials: "zw".to_string(),
            }
        );
    }

    #[test]
    fn build_search_keys_keeps_ascii_and_digits() {
        let keys = build_search_keys("Avatar 2");
        assert_eq!(keys.text, "avatar 2");
        assert_eq!(keys.pinyin, "avatar2");
        assert_eq!(keys.initials, "avatar2");
    }

    #[test]
    fn build_search_keys_normalizes_whitespace_and_symbols() {
        let keys = build_search_keys("  三体：The-Problem!!  ");
        assert_eq!(keys.text, "三体 the problem");
        assert_eq!(keys.pinyin, "santitheproblem");
        assert_eq!(keys.initials, "sttheproblem");
    }

    #[test]
    fn build_search_keys_handles_pinyin_input() {
        let keys = build_search_keys("  Zhen Huan ");
        assert_eq!(keys.text, "zhen huan");
        assert_eq!(keys.pinyin, "zhenhuan");
        assert_eq!(keys.initials, "zhenhuan");
    }

    #[test]
    fn empty_and_punctuation_only_inputs_are_ignored() {
        let keys = build_search_keys("  ---   ");
        assert_eq!(keys.text, "");
        assert_eq!(keys.pinyin, "");
        assert_eq!(keys.initials, "");
    }

    #[test]
    fn normalize_media_title_strips_year_and_resolution_suffix() {
        assert_eq!(
            normalize_media_title("疯狂海盗团2 (2025) - 2160p"),
            "疯狂海盗团2"
        );
    }

    #[test]
    fn normalize_media_title_handles_dot_separated_release_names() {
        assert_eq!(
            normalize_media_title("Movie.Title.2025.2160p.WEB-DL"),
            "Movie Title WEB-DL"
        );
    }

    #[test]
    fn normalize_media_title_keeps_non_suffix_year_titles() {
        assert_eq!(normalize_media_title("2012"), "2012");
    }

    #[test]
    fn normalize_media_title_strips_extensions() {
        assert_eq!(
            normalize_media_title("福音战士新剧场版：终 (2021) - 2160p.mkv"),
            "福音战士新剧场版：终"
        );
        assert_eq!(normalize_media_title("some_movie_2024.mp4"), "some movie");
        assert_eq!(normalize_media_title("movie.name.1080p.srt"), "movie name");
    }

    #[test]
    fn normalize_media_title_strips_release_groups() {
        assert_eq!(
            normalize_media_title("Movie Name [zmz] 1080p.mp4"),
            "Movie Name"
        );
    }
}
