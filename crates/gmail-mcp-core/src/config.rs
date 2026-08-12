//! Configuration loading, validation, and account resolution.
//!
//! The default configuration file is `~/.config/gmail-mcp.toml` (respecting
//! `XDG_CONFIG_HOME`). Accounts are named TOML tables; the table name is the
//! account alias.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::attachment::AttachmentPolicy;
use crate::error::{Error, account_not_found};

/// The default configuration file name.
pub const CONFIG_FILE_NAME: &str = "gmail-mcp.toml";

/// The default configuration path: `~/.config/gmail-mcp.toml`.
pub fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|| PathBuf::from(".config"))
        });
    base.join(CONFIG_FILE_NAME)
}

/// The parsed configuration file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub accounts: BTreeMap<String, AccountConfig>,
    #[serde(default)]
    pub attachments: AttachmentSettings,
}

/// Per-account configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub email: String,
    pub app_password: String,
    pub imap_host: String,
    #[serde(default = "default_port")]
    pub imap_port: u16,
    #[serde(default = "default_tls")]
    pub tls: bool,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub default: bool,
    /// Optional connection timeout in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout_secs: Option<u64>,
}

fn default_port() -> u16 {
    993
}

fn default_tls() -> bool {
    true
}

/// Attachment-related settings (all optional; defaults apply when absent).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttachmentSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_threshold_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u64>,
}

impl ConfigFile {
    /// Load the configuration from the default path.
    pub fn load() -> Result<ConfigFile, Error> {
        Self::load_from(&default_config_path())
    }

    /// Load the configuration from a specific path.
    pub fn load_from(path: &Path) -> Result<ConfigFile, Error> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::Config(format!(
                    "no configuration file at {}; run `gmail-mcp config add` to create one",
                    path.display()
                ))
            } else {
                Error::Config(format!("failed to read {}: {e}", path.display()))
            }
        })?;
        let config: ConfigFile = toml::from_str(&text)?;
        config.validate()?;
        Ok(config)
    }

    /// Save the configuration to a path, applying restrictive permissions.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Config(format!("failed to create {}: {e}", parent.display()))
            })?;
        }
        let text = toml::to_string(self)?;
        std::fs::write(path, text)
            .map_err(|e| Error::Config(format!("failed to write {}: {e}", path.display())))?;
        set_restrictive_permissions(path);
        Ok(())
    }

    /// Validate the whole configuration.
    pub fn validate(&self) -> Result<(), Error> {
        if self.accounts.is_empty() {
            return Err(Error::Config(
                "configuration contains no accounts; add one with `gmail-mcp config add`".into(),
            ));
        }
        let mut defaults = 0;
        for (alias, account) in &self.accounts {
            validate_alias(alias)?;
            account.validate(alias)?;
            if account.default {
                defaults += 1;
            }
        }
        if defaults > 1 {
            return Err(Error::Config(
                "multiple accounts are marked `default = true`; at most one is allowed".into(),
            ));
        }
        Ok(())
    }

    /// Resolve an account by alias, or the default account when `alias` is `None`.
    ///
    /// Returns the account and its alias.
    pub fn resolve_account(&self, alias: Option<&str>) -> Result<(&AccountConfig, String), Error> {
        match alias {
            Some(alias) => self
                .accounts
                .get(alias)
                .map(|a| (a, alias.to_string()))
                .ok_or_else(|| account_not_found(alias, &self.aliases())),
            None => {
                let default = self
                    .accounts
                    .iter()
                    .find(|(_, a)| a.default)
                    .map(|(alias, a)| (a, alias.clone()));
                match default {
                    Some((account, alias)) => Ok((account, alias)),
                    None => Err(Error::Config(
                        "no default account configured; specify an account alias".into(),
                    )),
                }
            }
        }
    }

    /// All account aliases, sorted.
    pub fn aliases(&self) -> Vec<String> {
        self.accounts.keys().cloned().collect()
    }

    /// Build the attachment policy from configuration settings.
    pub fn attachment_policy(&self) -> AttachmentPolicy {
        let mut policy = AttachmentPolicy::default();
        if let Some(dir) = &self.attachments.cache_dir {
            policy.cache_dir = dir.clone();
        }
        if let Some(mb) = self.attachments.direct_threshold_mb {
            policy.direct_threshold = mb * 1024 * 1024;
        }
        if let Some(hours) = self.attachments.retention_hours {
            policy.retention = std::time::Duration::from_secs(hours * 3600);
        }
        policy
    }
}

impl AccountConfig {
    /// Validate a single account.
    pub fn validate(&self, alias: &str) -> Result<(), Error> {
        if self.email.is_empty() {
            return Err(Error::Config(format!(
                "account `{alias}`: email is required"
            )));
        }
        if self.app_password.is_empty() {
            return Err(Error::Config(format!(
                "account `{alias}`: app_password is required"
            )));
        }
        if self.imap_host.is_empty() {
            return Err(Error::Config(format!(
                "account `{alias}`: imap_host is required"
            )));
        }
        if self.imap_port == 0 {
            return Err(Error::Config(format!(
                "account `{alias}`: imap_port must be 1-65535"
            )));
        }
        if let Some(tz) = &self.timezone {
            tz.parse::<chrono_tz::Tz>().map_err(|_| {
                Error::Config(format!("account `{alias}`: invalid timezone `{tz}`"))
            })?;
        }
        Ok(())
    }

    /// The account timezone, if configured.
    pub fn tz(&self) -> Option<chrono_tz::Tz> {
        self.timezone.as_deref().and_then(|n| n.parse().ok())
    }

    /// Connection timeout, defaulting to 30 seconds.
    pub fn connect_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.connect_timeout_secs.unwrap_or(30))
    }
}

fn validate_alias(alias: &str) -> Result<(), Error> {
    if alias.is_empty() || alias.contains('.') || alias.contains(' ') {
        return Err(Error::Config(format!(
            "invalid account alias `{alias}`: use a short name without dots or spaces"
        )));
    }
    Ok(())
}

/// Attempt to set restrictive permissions on the config file.
///
/// On platforms where this is not possible, the failure is ignored; callers
/// that care can check the resulting mode.
pub fn set_restrictive_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Whether the config file currently has restrictive permissions.
#[cfg(unix)]
pub fn has_restrictive_permissions(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o777 == 0o600)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
[accounts.personal]
email = "me@gmail.com"
app_password = "abcd efgh ijkl mnop"
imap_host = "imap.gmail.com"
imap_port = 993
tls = true
timezone = "America/New_York"
default = true
"#
    }

    #[test]
    fn parses_valid_config() {
        let config: ConfigFile = toml::from_str(sample_toml()).unwrap();
        config.validate().unwrap();
        let (account, alias) = config.resolve_account(None).unwrap();
        assert_eq!(alias, "personal");
        assert_eq!(account.email, "me@gmail.com");
        assert_eq!(account.tz(), Some(chrono_tz::Tz::America__New_York));
    }

    #[test]
    fn rejects_multiple_defaults() {
        let toml = r#"
[accounts.a]
email = "a@gmail.com"
app_password = "x"
imap_host = "imap.gmail.com"
default = true
[accounts.b]
email = "b@gmail.com"
app_password = "x"
imap_host = "imap.gmail.com"
default = true
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_invalid_timezone() {
        let toml = r#"
[accounts.a]
email = "a@gmail.com"
app_password = "x"
imap_host = "imap.gmail.com"
timezone = "Not/AZone"
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_missing_required_fields() {
        let toml = r#"
[accounts.a]
email = "a@gmail.com"
"#;
        // Missing required fields fail at deserialization.
        assert!(toml::from_str::<ConfigFile>(toml).is_err());
    }

    #[test]
    fn unknown_account_lists_aliases() {
        let config: ConfigFile = toml::from_str(sample_toml()).unwrap();
        let err = config.resolve_account(Some("nope")).unwrap_err();
        match err {
            Error::AccountNotFound(alias, available) => {
                assert_eq!(alias, "nope");
                assert!(available.contains("personal"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn no_default_requires_alias() {
        let toml = r#"
[accounts.a]
email = "a@gmail.com"
app_password = "x"
imap_host = "imap.gmail.com"
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        assert!(config.resolve_account(None).is_err());
        assert!(config.resolve_account(Some("a")).is_ok());
    }

    #[test]
    fn attachment_policy_from_settings() {
        let toml = r#"
[accounts.a]
email = "a@gmail.com"
app_password = "x"
imap_host = "imap.gmail.com"
[attachments]
direct_threshold_mb = 1
retention_hours = 2
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let policy = config.attachment_policy();
        assert_eq!(policy.direct_threshold, 1024 * 1024);
        assert_eq!(policy.retention, std::time::Duration::from_secs(7200));
        assert_eq!(policy.cache_dir, crate::attachment::default_cache_dir());
    }

    #[test]
    fn defaults_apply() {
        let toml = r#"
[accounts.a]
email = "a@gmail.com"
app_password = "x"
imap_host = "imap.gmail.com"
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let account = &config.accounts["a"];
        assert_eq!(account.imap_port, 993);
        assert!(account.tls);
        assert!(!account.default);
        assert_eq!(
            account.connect_timeout(),
            std::time::Duration::from_secs(30)
        );
    }

    #[test]
    fn save_round_trips() {
        let config: ConfigFile = toml::from_str(sample_toml()).unwrap();
        let dir = std::env::temp_dir().join(format!("gmail-mcp-cfg-{}", std::process::id()));
        let path = dir.join("gmail-mcp.toml");
        config.save(&path).unwrap();
        let loaded = ConfigFile::load_from(&path).unwrap();
        assert_eq!(loaded.accounts.len(), 1);
        assert_eq!(loaded.accounts["personal"].email, "me@gmail.com");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
