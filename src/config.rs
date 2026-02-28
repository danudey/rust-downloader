use std::collections::HashMap;

use config::{Config, File, FileFormat};
use log::{debug, info, warn};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub domains: HashMap<String, DomainConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DefaultsConfig {
    pub browser: Option<String>,
    #[serde(default = "default_true")]
    pub cookies: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            browser: None,
            cookies: true,
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DomainConfig {
    pub browser: Option<String>,
    pub cookies: Option<bool>,
}

impl AppConfig {
    pub fn load() -> Self {
        let dirs = xdg::BaseDirectories::with_prefix("rustdl");

        let config_path = match dirs.find_config_file("config.toml") {
            Some(path) => path,
            None => {
                debug!("No config file found at rustdl/config.toml");
                return Self::default();
            }
        };

        info!("Loading config from: {}", config_path.display());

        match Config::builder()
            .add_source(File::from(config_path.as_path()).format(FileFormat::Toml))
            .build()
        {
            Ok(settings) => match settings.try_deserialize() {
                Ok(config) => {
                    debug!("Successfully parsed config file");
                    config
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize config file {}: {}",
                        config_path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                warn!(
                    "Failed to load config file {}: {}",
                    config_path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    pub fn cookies_enabled_for_domain(&self, domain: &str) -> bool {
        if let Some(domain_config) = self.domains.get(domain) {
            if let Some(cookies) = domain_config.cookies {
                return cookies;
            }
        }
        self.defaults.cookies
    }

    pub fn browser_for_domain(&self, domain: &str) -> Option<&str> {
        if let Some(domain_config) = self.domains.get(domain) {
            if domain_config.browser.is_some() {
                return domain_config.browser.as_deref();
            }
        }
        self.defaults.browser.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_toml(toml_str: &str) -> AppConfig {
        Config::builder()
            .add_source(File::from_str(toml_str, FileFormat::Toml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(config.defaults.cookies);
        assert!(config.defaults.browser.is_none());
        assert!(config.domains.is_empty());
    }

    #[test]
    fn test_cookies_enabled_for_domain_no_override() {
        let config = AppConfig::default();
        assert!(config.cookies_enabled_for_domain("example.com"));
    }

    #[test]
    fn test_cookies_enabled_for_domain_with_override() {
        let mut config = AppConfig::default();
        config.domains.insert(
            "example.com".to_string(),
            DomainConfig {
                cookies: Some(false),
                browser: None,
            },
        );
        assert!(!config.cookies_enabled_for_domain("example.com"));
        assert!(config.cookies_enabled_for_domain("other.com"));
    }

    #[test]
    fn test_cookies_enabled_respects_global_default() {
        let mut config = AppConfig::default();
        config.defaults.cookies = false;
        assert!(!config.cookies_enabled_for_domain("example.com"));
    }

    #[test]
    fn test_cookies_domain_override_trumps_global() {
        let mut config = AppConfig::default();
        config.defaults.cookies = false;
        config.domains.insert(
            "example.com".to_string(),
            DomainConfig {
                cookies: Some(true),
                browser: None,
            },
        );
        assert!(config.cookies_enabled_for_domain("example.com"));
        assert!(!config.cookies_enabled_for_domain("other.com"));
    }

    #[test]
    fn test_browser_for_domain_no_override() {
        let config = AppConfig::default();
        assert_eq!(config.browser_for_domain("example.com"), None);
    }

    #[test]
    fn test_browser_for_domain_global_default() {
        let mut config = AppConfig::default();
        config.defaults.browser = Some("firefox".to_string());
        assert_eq!(config.browser_for_domain("example.com"), Some("firefox"));
    }

    #[test]
    fn test_browser_for_domain_with_override() {
        let mut config = AppConfig::default();
        config.defaults.browser = Some("firefox".to_string());
        config.domains.insert(
            "example.com".to_string(),
            DomainConfig {
                browser: Some("chrome".to_string()),
                cookies: None,
            },
        );
        assert_eq!(config.browser_for_domain("example.com"), Some("chrome"));
        assert_eq!(config.browser_for_domain("other.com"), Some("firefox"));
    }

    #[test]
    fn test_domain_config_none_cookies_falls_through() {
        let mut config = AppConfig::default();
        config.domains.insert(
            "example.com".to_string(),
            DomainConfig {
                browser: Some("chrome".to_string()),
                cookies: None,
            },
        );
        assert!(config.cookies_enabled_for_domain("example.com"));
    }

    #[test]
    fn test_parse_full_toml() {
        let config = parse_toml(
            r#"
[defaults]
browser = "firefox"
cookies = true

[domains."example.com"]
browser = "chrome"
cookies = true

[domains."public-files.org"]
cookies = false
"#,
        );
        assert_eq!(config.defaults.browser, Some("firefox".to_string()));
        assert!(config.defaults.cookies);
        assert_eq!(config.browser_for_domain("example.com"), Some("chrome"));
        assert!(config.cookies_enabled_for_domain("example.com"));
        assert!(!config.cookies_enabled_for_domain("public-files.org"));
        assert_eq!(
            config.browser_for_domain("public-files.org"),
            Some("firefox")
        );
    }

    #[test]
    fn test_parse_minimal_toml() {
        let config = parse_toml("");
        assert!(config.defaults.cookies);
        assert!(config.defaults.browser.is_none());
        assert!(config.domains.is_empty());
    }

    #[test]
    fn test_parse_defaults_only_toml() {
        let config = parse_toml(
            r#"
[defaults]
browser = "edge"
cookies = false
"#,
        );
        assert_eq!(config.defaults.browser, Some("edge".to_string()));
        assert!(!config.defaults.cookies);
    }

    #[test]
    fn test_load_returns_defaults_on_missing_file() {
        let config = AppConfig::load();
        assert!(config.defaults.cookies);
    }
}
