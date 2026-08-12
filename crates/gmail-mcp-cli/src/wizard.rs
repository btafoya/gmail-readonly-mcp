//! Interactive configuration wizard for adding accounts.

use dialoguer::{Confirm, Input, Password};
use gmail_mcp_core::config::{AccountConfig, ConfigFile, default_config_path};
use gmail_mcp_core::error::Error;

/// Interactively add a named account to the configuration.
pub fn add_account() -> Result<(), Error> {
    let path = default_config_path();
    let mut config = if path.exists() {
        ConfigFile::load()?
    } else {
        ConfigFile::default()
    };

    let alias: String = Input::new()
        .with_prompt("Account alias (e.g. personal)")
        .interact_text()
        .map_err(wizard_err)?;
    let email: String = Input::new()
        .with_prompt("Gmail address")
        .interact_text()
        .map_err(wizard_err)?;
    let app_password: String = Password::new()
        .with_prompt("App password (16 characters, no spaces)")
        .interact()
        .map_err(wizard_err)?;
    let imap_host: String = Input::new()
        .with_prompt("IMAP host")
        .default("imap.gmail.com".to_string())
        .interact_text()
        .map_err(wizard_err)?;
    let imap_port: u16 = Input::new()
        .with_prompt("IMAP port")
        .default(993)
        .interact_text()
        .map_err(wizard_err)?;
    let tls: bool = Confirm::new()
        .with_prompt("Use TLS?")
        .default(true)
        .interact()
        .map_err(wizard_err)?;
    let timezone: String = Input::new()
        .with_prompt("Timezone (optional, e.g. America/New_York)")
        .allow_empty(true)
        .interact_text()
        .map_err(wizard_err)?;
    let is_default: bool = Confirm::new()
        .with_prompt("Set as the default account?")
        .default(config.accounts.is_empty())
        .interact()
        .map_err(wizard_err)?;

    let account = AccountConfig {
        email,
        app_password,
        imap_host,
        imap_port,
        tls,
        timezone: {
            let tz = timezone.trim();
            if tz.is_empty() {
                None
            } else {
                Some(tz.to_string())
            }
        },
        default: is_default,
        connect_timeout_secs: None,
    };
    account.validate(&alias)?;
    config.accounts.insert(alias, account);
    config.validate()?;
    config.save(&path)?;
    println!("saved configuration to {}", path.display());
    Ok(())
}

fn wizard_err(e: dialoguer::Error) -> Error {
    Error::Internal(format!("wizard error: {e}"))
}
