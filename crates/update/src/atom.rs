use crate::version::{is_version, parse_version};

const TAG_PREFIX: &str = "https://github.com/BeafJerky/LightProxy/releases/tag/v";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEntry {
    pub version: String,
    pub url: String,
}

pub fn parse_releases(xml: &str) -> Vec<ReleaseEntry> {
    let mut entries: Vec<ReleaseEntry> = Vec::new();
    for (index, _) in xml.match_indices("href=\"") {
        let rest = &xml[index + 6..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        let url = &rest[..end];
        let Some(version) = url.strip_prefix(TAG_PREFIX) else {
            continue;
        };
        if !is_version(version) || entries.iter().any(|entry| entry.version == version) {
            continue;
        }
        entries.push(ReleaseEntry {
            version: version.to_string(),
            url: url.to_string(),
        });
    }
    entries
}

pub fn latest(entries: &[ReleaseEntry]) -> Option<&ReleaseEntry> {
    entries.iter().max_by(|left, right| {
        parse_version(&left.version)
            .unwrap_or((0, 0, 0))
            .cmp(&parse_version(&right.version).unwrap_or((0, 0, 0)))
    })
}
