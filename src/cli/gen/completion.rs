use std::path::Path;
use std::str::FromStr;

use color_eyre::eyre::Result;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use indoc::printdoc;

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

        // println!("{shell:?}");

        match shell {
            Some(shell) => generate(shell, &mut cmd, name, &mut std::io::stdout()),
            None => {
                printdoc! {"
                    Couldn't detect shell.
                    Provide shell as an argument to the command, ex.

                        dolores gen completion bash
                "};
            },
        }

        Ok(())
    }

    fn default_shell() -> Option<Shell> {
        let shell_env = std::env::var("SHELL").ok()?;
        let shell_path = Path::new(&shell_env);

        shell_path
            .file_stem()?
            .to_str()
            .and_then(|x| FromStr::from_str(x).ok())
    }
}
