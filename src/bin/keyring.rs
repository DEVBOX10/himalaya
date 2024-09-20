use std::{collections::HashMap, env};

use email::{
    account::config::passwd::PasswdConfig,
    imap::config::{ImapAuthConfig, ImapConfig},
};
use himalaya::{
    account::{
        arg::name::AccountNameArg, command::configure::AccountConfigureCommand,
        config::TomlAccountConfig,
    },
    config::Config,
    output::OutputFmt,
    printer::StdoutPrinter,
};
use pimalaya_tui::tracing::Tracing;
use secret::{keyring::KeyringEntry, Secret};
use tracing::info;

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", "debug");
    Tracing::install().unwrap();

    secret::keyring::set_global_service_name("himalaya-cli");

    info!("checking keyring-lib");

    let entry = KeyringEntry::try_new("key").unwrap();
    entry.set_secret("val").await.unwrap();
    assert_eq!("val", entry.get_secret().await.unwrap());

    info!("checking secret-lib");

    let mut secret = Secret::new_keyring_entry(entry);
    assert_eq!(secret.get().await.unwrap(), "val");

    secret.set("val2").await.unwrap();
    assert_eq!(secret.get().await.unwrap(), "val2");

    info!("checking email-lib");

    let config = PasswdConfig(secret);
    config.reset().await.unwrap();
    config.configure(|| Ok(String::from("val3"))).await.unwrap();
    assert_eq!(config.get().await.unwrap(), "val3");

    info!("checking himalaya");

    let mut printer = StdoutPrinter::new(OutputFmt::Plain);
    let cmd = AccountConfigureCommand {
        account: AccountNameArg {
            name: String::from("account"),
        },
        reset: true,
    };

    cmd.execute(
        &mut printer,
        &Config {
            accounts: HashMap::from_iter([(
                String::from("account"),
                TomlAccountConfig {
                    imap: Some(ImapConfig {
                        auth: ImapAuthConfig::Passwd(config.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let secret = config.get().await.unwrap();
    println!("secret: {secret}");
}
