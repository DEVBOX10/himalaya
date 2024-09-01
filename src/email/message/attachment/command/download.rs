use clap::Parser;
use color_eyre::{eyre::Context, Result};
use email::backend::feature::BackendFeatureSource;
use std::{fs, path::PathBuf};
use tracing::info;
use uuid::Uuid;

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::Config,
    envelope::arg::ids::EnvelopeIdsArgs, folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Download all attachments for the given message.
///
/// This command allows you to download all attachments found for the
/// given message to your downloads directory.
#[derive(Debug, Parser)]
pub struct AttachmentDownloadCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl AttachmentDownloadCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &Config) -> Result<()> {
        info!("executing download attachment(s) command");

        let folder = &self.folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref())?;

        let get_messages_kind = toml_account_config.get_messages_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config.clone(),
            get_messages_kind,
            |builder| builder.set_get_messages(BackendFeatureSource::Context),
        )
        .await?;

        let emails = backend.get_messages(folder, ids).await?;

        let mut emails_count = 0;
        let mut attachments_count = 0;

        let mut ids = ids.iter();
        for email in emails.to_vec() {
            let id = ids.next().unwrap();
            let attachments = email.attachments()?;

            if attachments.is_empty() {
                printer.log(format!("No attachment found for message {id}!"))?;
                continue;
            } else {
                emails_count += 1;
            }

            printer.log(format!(
                "{} attachment(s) found for message {id}!",
                attachments.len()
            ))?;

            for attachment in attachments {
                let filename: PathBuf = attachment
                    .filename
                    .unwrap_or_else(|| Uuid::new_v4().to_string())
                    .into();
                let filepath = account_config.get_download_file_path(&filename)?;
                printer.log(format!("Downloading {:?}…", filepath))?;
                fs::write(&filepath, &attachment.body)
                    .with_context(|| format!("cannot save attachment at {filepath:?}"))?;
                attachments_count += 1;
            }
        }

        match attachments_count {
            0 => printer.out("No attachment found!"),
            1 => printer.out("Downloaded 1 attachment!"),
            n => printer.out(format!(
                "Downloaded {} attachment(s) from {} messages(s)!",
                n, emails_count,
            )),
        }
    }
}
