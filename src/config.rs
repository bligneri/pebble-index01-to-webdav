use std::collections::BTreeMap;

use anyhow::{Context, bail};
use chrono_tz::Tz;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub caldav: Caldav,
    pub timezone: Tz,
    pub journal: Journal,
    pub types: Types,
    pub default_domain: String,
    pub domains: BTreeMap<String, Domain>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caldav {
    pub base_url: String,
    pub user: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub mode: JournalMode,
}

// ponytail: only append for now, `new` (one entry per dictation) comes later.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JournalMode {
    Append,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Types {
    pub note: Vec<String>,
    pub journal: Vec<String>,
    pub todo: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Domain {
    pub collection: String,
    pub keywords: Vec<String>,
}

pub struct Secrets {
    pub webhook: String,
    // optional until CalDAV writes are wired in (plan step 6)
    pub caldav_password: Option<String>,
}

impl Config {
    pub fn parse(yaml: &str) -> anyhow::Result<Self> {
        let cfg: Config = serde_yaml::from_str(yaml)?;
        if !cfg.domains.contains_key(&cfg.default_domain) {
            bail!("default_domain {:?} is not in domains", cfg.default_domain);
        }
        if !cfg.caldav.base_url.ends_with('/') {
            bail!("caldav.base_url must end with /");
        }
        Ok(cfg)
    }

    pub fn load(path: &str) -> anyhow::Result<Self> {
        let yaml = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
        Self::parse(&yaml).with_context(|| format!("parsing {path}"))
    }
}

impl Secrets {
    pub fn from_env() -> anyhow::Result<Self> {
        let get = |k: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.is_empty())
                .with_context(|| format!("{k} is not set"))
        };
        Ok(Self {
            webhook: get("PEBBLE_WEBHOOK_SECRET")?,
            caldav_password: get("CALDAV_APP_PASSWORD").ok(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses() {
        let cfg = Config::parse(include_str!("../config.example.yaml")).unwrap();
        assert_eq!(cfg.default_domain, "perso");
        assert_eq!(cfg.domains["esa"].keywords, ["esa", "travail"]);
        assert_eq!(cfg.journal.mode, JournalMode::Append);
    }

    #[test]
    fn rejects_unknown_default_domain_and_mode() {
        let yaml = include_str!("../config.example.yaml");
        assert!(
            Config::parse(&yaml.replace("default_domain: perso", "default_domain: x")).is_err()
        );
        assert!(Config::parse(&yaml.replace("mode: append", "mode: new")).is_err());
    }
}
