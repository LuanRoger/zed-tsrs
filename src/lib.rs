// Adapted from zed-extensions/tsgo (Apache-2.0); modified for tsrs.
mod installer;
mod platform;
mod settings;

use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree, settings::LspSettings};

struct TsrsExtension;

impl zed::Extension for TsrsExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let settings = LspSettings::for_worktree(id.as_ref(), worktree)?;
        let binary = settings.binary.unwrap_or(zed::settings::CommandSettings {
            path: None,
            arguments: None,
            env: None,
        });
        // Resolve settings on every launch: one extension instance can serve
        // worktrees with different overrides and managed version pins.
        let command = match settings::binary_source(binary.path, || worktree.which("tsrs"))? {
            settings::BinarySource::Local(path) => path,
            settings::BinarySource::Managed => {
                let version = settings::package_version(settings.settings.as_ref())?;
                installer::managed_binary(id, version)?
            }
        };
        Ok(zed::Command {
            command,
            args: binary
                .arguments
                .unwrap_or_else(|| vec!["--lsp".into(), "-stdio".into()]),
            env: binary.env.into_iter().flatten().collect(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(id.as_ref(), worktree)?.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(settings::workspace_configuration(
            LspSettings::for_worktree(id.as_ref(), worktree)?.settings,
        ))
    }
}

zed::register_extension!(TsrsExtension);
