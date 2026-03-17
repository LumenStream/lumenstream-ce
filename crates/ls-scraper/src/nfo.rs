use std::path::Path;

use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::model::{ScrapeExternalIds, ScrapeMatchHints};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NfoItemKind {
    Movie,
    TvShow,
    Season,
    Episode,
}

impl NfoItemKind {
    fn root_tag(&self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::TvShow => "tvshow",
            Self::Season => "season",
            Self::Episode => "episodedetails",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct NfoSidecarHint {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub plot: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub aired: Option<String>,
    #[serde(default)]
    pub show_title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub studios: Vec<String>,
    #[serde(default)]
    pub external_ids: ScrapeExternalIds,
}

impl NfoSidecarHint {
    pub fn to_match_hints(&self) -> ScrapeMatchHints {
        ScrapeMatchHints {
            title: self.title.clone(),
            year: self.year,
            external_ids: self.external_ids.clone(),
            tags: self.tags.clone(),
            ..ScrapeMatchHints::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct NfoDocument {
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub plot: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub aired: Option<String>,
    #[serde(default)]
    pub show_title: Option<String>,
    #[serde(default)]
    pub tmdb_id: Option<String>,
    #[serde(default)]
    pub imdb_id: Option<String>,
    #[serde(default)]
    pub tvdb_id: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub studios: Vec<String>,
}

pub fn read_nfo_sidecar_hints(path: &Path) -> Option<NfoSidecarHint> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(parse_nfo_sidecar_hints(&content))
}

fn parse_nfo_sidecar_hints(content: &str) -> NfoSidecarHint {
    let sanitized = strip_actor_blocks(content);
    NfoSidecarHint {
        title: capture_tag(&sanitized, "title"),
        plot: capture_tag(&sanitized, "plot"),
        year: capture_tag(&sanitized, "year").and_then(|value| value.parse::<i32>().ok()),
        aired: capture_tag(&sanitized, "aired"),
        show_title: capture_tag(&sanitized, "showtitle"),
        tags: capture_tags(&sanitized, "tag"),
        genres: capture_tags(&sanitized, "genre"),
        studios: capture_tags(&sanitized, "studio"),
        external_ids: ScrapeExternalIds {
            tmdb: capture_unique_id(&sanitized, "tmdb")
                .or_else(|| capture_tag(&sanitized, "tmdbid")),
            imdb: capture_unique_id(&sanitized, "imdb")
                .or_else(|| capture_tag(&sanitized, "imdbid"))
                .or_else(|| capture_tag(&sanitized, "imdb_id")),
            tvdb: capture_unique_id(&sanitized, "tvdb")
                .or_else(|| capture_tag(&sanitized, "tvdbid")),
            bangumi: None,
            extra: Default::default(),
        },
    }
}

pub fn build_nfo_document(kind: NfoItemKind, hint: &NfoSidecarHint) -> NfoDocument {
    NfoDocument {
        kind: kind.root_tag().to_string(),
        title: hint.title.clone(),
        plot: hint.plot.clone(),
        year: hint.year,
        aired: hint.aired.clone(),
        show_title: hint.show_title.clone(),
        tmdb_id: hint.external_ids.tmdb.clone(),
        imdb_id: hint.external_ids.imdb.clone(),
        tvdb_id: hint.external_ids.tvdb.clone(),
        genres: hint.genres.clone(),
        tags: hint.tags.clone(),
        studios: hint.studios.clone(),
    }
}

pub fn render_nfo_document(document: &NfoDocument) -> anyhow::Result<String> {
    if document.kind.trim().is_empty() {
        anyhow::bail!("nfo document kind is required");
    }

    let mut lines = vec![format!("<{}>", document.kind)];
    push_tag(&mut lines, "title", document.title.as_deref());
    push_tag(&mut lines, "plot", document.plot.as_deref());
    if let Some(year) = document.year {
        lines.push(format!("  <year>{year}</year>"));
    }
    push_tag(&mut lines, "aired", document.aired.as_deref());
    push_tag(&mut lines, "showtitle", document.show_title.as_deref());
    push_tag(&mut lines, "tmdbid", document.tmdb_id.as_deref());
    push_unique_id(&mut lines, "tmdb", document.tmdb_id.as_deref(), true);
    push_tag(&mut lines, "imdbid", document.imdb_id.as_deref());
    push_unique_id(&mut lines, "imdb", document.imdb_id.as_deref(), false);
    push_tag(&mut lines, "tvdbid", document.tvdb_id.as_deref());
    push_unique_id(&mut lines, "tvdb", document.tvdb_id.as_deref(), false);
    for genre in &document.genres {
        push_tag(&mut lines, "genre", Some(genre.as_str()));
    }
    for tag in &document.tags {
        push_tag(&mut lines, "tag", Some(tag.as_str()));
    }
    for studio in &document.studios {
        push_tag(&mut lines, "studio", Some(studio.as_str()));
    }
    lines.push(format!("</{}>", document.kind));
    Ok(lines.join(
        "
",
    ) + "
")
}

pub fn write_nfo_document(path: &Path, document: &NfoDocument) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create nfo dir: {}", parent.display()))?;
    }
    std::fs::write(path, render_nfo_document(document)?)
        .with_context(|| format!("failed to write nfo: {}", path.display()))
}

fn push_tag(lines: &mut Vec<String>, tag: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    lines.push(format!("  <{tag}>{}</{tag}>", xml_escape(value)));
}

fn push_unique_id(lines: &mut Vec<String>, kind: &str, value: Option<&str>, default: bool) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    lines.push(format!(
        "  <uniqueid type=\"{kind}\" default=\"{}\">{}</uniqueid>",
        if default { "true" } else { "false" },
        xml_escape(value)
    ));
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn capture_tag(content: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?is)<{tag}>\s*(.*?)\s*</{tag}>");
    Regex::new(&pattern)
        .ok()?
        .captures(content)?
        .get(1)
        .and_then(|matched| normalize_value(matched.as_str()))
}

fn capture_unique_id(content: &str, id_type: &str) -> Option<String> {
    let pattern =
        format!(r#"(?is)<uniqueid[^>]*type=[\"']{id_type}[\"'][^>]*>\s*(.*?)\s*</uniqueid>"#);
    Regex::new(&pattern)
        .ok()?
        .captures(content)?
        .get(1)
        .and_then(|matched| normalize_value(matched.as_str()))
}

fn capture_tags(content: &str, tag: &str) -> Vec<String> {
    let pattern = format!(r"(?is)<{tag}>\s*(.*?)\s*</{tag}>");
    let Some(re) = Regex::new(&pattern).ok() else {
        return Vec::new();
    };
    re.captures_iter(content)
        .filter_map(|capture| {
            capture
                .get(1)
                .and_then(|matched| normalize_value(matched.as_str()))
        })
        .collect()
}

fn normalize_value(raw: &str) -> Option<String> {
    let value = raw
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .trim()
        .to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn strip_actor_blocks(content: &str) -> String {
    Regex::new(r"(?is)<actor>.*?</actor>")
        .ok()
        .map(|re| re.replace_all(content, "").to_string())
        .unwrap_or_else(|| content.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        NfoItemKind, NfoSidecarHint, build_nfo_document, read_nfo_sidecar_hints,
        render_nfo_document,
    };
    use crate::model::ScrapeExternalIds;

    #[test]
    fn parse_sidecar_hints_extracts_known_ids() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("movie.nfo");
        std::fs::write(
            &path,
            r#"<movie><title>Interstellar</title><year>2014</year><tmdbid>157336</tmdbid><uniqueid type="imdb" default="false">tt0816692</uniqueid></movie>"#,
        )
        .expect("write nfo");

        let hint = read_nfo_sidecar_hints(&path).expect("hint");
        assert_eq!(hint.title.as_deref(), Some("Interstellar"));
        assert_eq!(hint.year, Some(2014));
        assert_eq!(hint.external_ids.tmdb.as_deref(), Some("157336"));
        assert_eq!(hint.external_ids.imdb.as_deref(), Some("tt0816692"));
    }

    #[test]
    fn render_document_preserves_unique_ids() {
        let hint = NfoSidecarHint {
            title: Some("The Last of Us".to_string()),
            external_ids: ScrapeExternalIds {
                tmdb: Some("100088".to_string()),
                imdb: Some("tt3581920".to_string()),
                tvdb: Some("392256".to_string()),
                bangumi: None,
                extra: Default::default(),
            },
            ..NfoSidecarHint::default()
        };
        let document = build_nfo_document(NfoItemKind::TvShow, &hint);
        let xml = render_nfo_document(&document).expect("xml");
        assert!(xml.contains("<uniqueid type=\"tmdb\" default=\"true\">100088</uniqueid>"));
        assert!(xml.contains("<uniqueid type=\"imdb\" default=\"false\">tt3581920</uniqueid>"));
        assert!(xml.contains("<tvdbid>392256</tvdbid>"));
    }
}
