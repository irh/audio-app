use anyhow::{Result, bail};
use std::iter::Peekable;

const HELP: &str = "\
cargo xtask plugin

Build plugins using nih_plug.

Usage:
  cargo xtask plugin <editor> <format> [flags]

Ui:
  none: Builds the plugins with no editor.
  egui: Builds the plugin with an egui editor.

Formats:
  all: Build all available plugin formats.
  clap: Build a CLAP plugin.
  vst3: Build a VST3 plugin.
";

pub fn command(mut args: Peekable<impl Iterator<Item = String>>) -> Result<()> {
    let (package, package_features) = match args.next().as_deref() {
        Some("egui") => ("plugin_egui", None),
        Some("none") => ("plugin", Some("export_no_editor")),
        Some("help") | Some("--help") | None => {
            println!("{HELP}");
            return Ok(());
        }
        Some(unexpected) => {
            bail!(
                "unexpected editor {unexpected}.

{HELP}"
            );
        }
    };

    match args.next().as_deref() {
        Some("all") => nih_plug_xtask::main_with_args(
            "cargo xtask all",
            make_nih_args(args, package, package_features, "clap,vst3"),
        ),
        Some("clap") => nih_plug_xtask::main_with_args(
            "cargo xtask clap",
            make_nih_args(args, package, package_features, "clap"),
        ),
        Some("vst3") => nih_plug_xtask::main_with_args(
            "cargo xtask vst3",
            make_nih_args(args, package, package_features, "vst3"),
        ),
        Some("--help") | None => {
            println!("{HELP}");
            Ok(())
        }
        Some(unexpected) => {
            bail!(
                "unexpected format {unexpected}.

{HELP}"
            );
        }
    }
}

fn make_nih_args(
    args: impl Iterator<Item = String>,
    package: &str,
    editor_features: Option<&str>,
    format_features: &str,
) -> Vec<String> {
    let mut result = vec!["bundle".into(), package.into()];
    result.extend(args);
    if let Some(editor_features) = editor_features {
        result.extend(["--features".into(), editor_features.into()]);
    }
    result.extend(["--features".into(), format_features.into()]);
    result
}
