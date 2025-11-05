mod plugin;

use anyhow::{Result, bail};

const HELP: &str = "\
Available commands:
  plugin: Build a plugin

Run `cargo xtask <command> --help` for more information about a command.
";

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1).peekable();

    match args.next().as_deref() {
        Some("plugin") => plugin::command(args),
        Some("help") | Some("--help") | None => {
            println!("{HELP}");
            Ok(())
        }
        Some(unexpected) => {
            bail!(
                "unexpected command {unexpected}.

{HELP}"
            );
        }
    }
}
