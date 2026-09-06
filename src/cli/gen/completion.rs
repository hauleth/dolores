use std::path::Path;
use std::str::FromStr;

use color_eyre::eyre::Result;

use clap::CommandFactory;
use clap::ValueEnum;
use clap_complete::{generate, Shell};

use indoc::eprintdoc;

#[derive(Debug)]
struct UnsupportedShell;

impl std::fmt::Display for UnsupportedShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Unknown shell")
    }
}

impl std::error::Error for UnsupportedShell {}

/// Generate shell completion
#[derive(clap::Args, Debug)]
pub(crate) struct Command {
    /// Name of the shell for which the completion should be generated.
    ///
    /// By default will try to detect the shell using `SHELL` environment variable.
    shell: Option<Shell>,
}

impl Command {
    pub fn run(self) -> Result<()> {
        let mut cmd = crate::cli::App::command();
        let name = cmd.get_name().to_string();

        let shell = self.shell.or_else(Self::default_shell);

        match shell {
            Some(shell) => {
                generate(shell, &mut cmd, name, &mut std::io::stdout());

                Ok(())
            }
            None => {
                let shells = Shell::value_variants()
                    .iter()
                    .map(|v| v.to_possible_value().unwrap().get_name().to_owned())
                    .collect::<Box<[_]>>()
                    .join(", ");

                eprintdoc! {"
                    Couldn't detect shell.
                    Provide shell as an argument to the command, ex.

                        dolores gen completion bash

                    Supported shells: {shells}
                "};

                Err(UnsupportedShell)?
            }
        }
    }

    /// Check `SHELL` environment variable try to detect current shell.
    fn default_shell() -> Option<Shell> {
        let shell_env = std::env::var("SHELL").ok()?;
        let shell_path = Path::new(&shell_env);

        shell_path
            .file_stem()?
            .to_str()
            .and_then(|x| FromStr::from_str(x).ok())
    }
}
