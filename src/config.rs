use std::env::current_dir;
use std::fmt::Display;
use anyhow::{anyhow, Context};
use figment::providers::{Env, Format, Toml};
use figment::Figment;
use microxdg::XdgApp;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use actix_web::http::header::LanguageTag;
use tracing::debug;
use url::Url;
use uuid::Uuid;

/// The locale of the calendar
#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum Locale {
    /// German locale
    de,
    /// English locale
    en,
}

impl Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::de => write!(f, "de"),
            Self::en => write!(f, "en"),
        }
    }
}

impl FromStr for Locale {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "de" => Ok(Self::de),
            "en" => Ok(Self::en),
            _ => Err(anyhow!("Could not parse {s} into a Locale")),
        }
    }
}

impl Into<LanguageTag> for Locale {
    fn into(self) -> LanguageTag {
        self.to_string().parse().unwrap()
    }
}

fn default_endpoint() -> Url {
    "https://tiss.tuwien.ac.at/events/rest/calendar/personal"
        .parse()
        .unwrap()
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TissConfig {
    /// Individual TISS token link components
    Components {
        /// The TISS endpoint to get the icalendar file from
        #[serde(default = "default_endpoint")]
        endpoint: Url,
        /// The locale of the calendar
        locale: Locale,
        /// The token used for auth
        token: Uuid,
    },
    /// A token link from TISS
    Link(Url),
}

impl TissConfig {
    pub fn link(&self) -> Url {
        match self {
            Self::Link(link) => link.clone(),
            Self::Components { endpoint, locale, token } => {
                let mut link = endpoint.clone();
                link.query_pairs_mut()
                    .append_pair("locale", &locale.to_string())
                    .append_pair("token", &token.to_string());
                link
            }
        }
    }

    pub fn locale(&self) -> anyhow::Result<Locale> {
        match self {
            Self::Link(link) => {
                link.query_pairs()
                    .find(|(key, _)| *key == "locale")
                    .ok_or(anyhow!("Could not find locale query parameter in tiss token link"))?
                    .0
                    .parse()
            },
            Self::Components { locale, .. } => Ok(*locale),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub bind: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8485".to_string()
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub tiss: TissConfig,
}

/// TU Planner configuration
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub app: AppConfig,
    #[serde(default)]
    pub service: ServiceConfig,
}

impl Config {
    fn get_config_potential_paths() -> anyhow::Result<Vec<PathBuf>> {
        let dirs = XdgApp::new("tu-planner")?;
        let config_file = dirs.app_config()?;
        let sys_configs = dirs.app_sys_config()?;
        let sys_config_files = sys_configs.into_iter()
            .chain([config_file, current_dir()?].into_iter());

        Ok(sys_config_files.collect())
    }

    pub fn load() -> anyhow::Result<Self> {
        let mut figment = Figment::new();

        let paths = Self::get_config_potential_paths()
            .context("Failed to find potential config paths")?;

        debug!(paths = ?paths, "searching potential configuration paths");

        for path in paths {
            let path = path.join("config.toml");
            if path.exists() {
                figment = figment.merge(Toml::file_exact(path));
            }
        }

        figment = figment.merge(Env::prefixed("TU_PLANNER"));

        let config = figment.extract()?;
        Ok(config)
    }
}
