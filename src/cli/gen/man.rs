use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use color_eyre::eyre::Result;

use clap::CommandFactory;
use clap_mangen::Man;

/// Generate manpages for Dolores
#[derive(clap::Args, Debug)]
pub(crate) struct Command {
    /// Directory where manpages will be written to.
    output_dir: std::path::PathBuf,
}

impl Command {
    pub fn run(self) -> Result<()> {
        let mut cmd = crate::cli::App::command();
        cmd.build();

        print_manpages(&self.output_dir, &cmd)?;

        Ok(())
    }
}

fn print_manpages(dir: &Path, app: &clap::Command) -> Result<()> {
    fn print(dir: &Path, app: &clap::Command) -> Result<()> {
        // `get_display_name()` is `Some` for all instances, except the root.
        let name = app.get_display_name().unwrap_or_else(|| app.get_name());

        {
            let mut out = File::create(dir.join(format!("{name}.1")))?;

            Man::new(app.clone()).render(&mut out)?;
            out.flush()?;
        }

        for sub in app.get_subcommands() {
            print(dir, sub)?;
        }

        Ok(())
    }

    print(dir, &app)
}
