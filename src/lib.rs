pub mod build;
pub mod devices;
pub mod init;
pub mod metadata;
pub mod run;

use clap::Parser;

#[derive(Parser)]
#[clap(styles = clap_cargo::style::CLAP_STYLING)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,
}

impl Args {
    pub fn run(self) -> eyre::Result<()> {
        match self.command {
            Command::Init(init) => init.run(),
            Command::Devices(devices) => devices.run(),
            Command::Build(build) => build.run(),
            Command::Run(run) => run.run(),
        }
    }
}

#[derive(Parser)]
pub enum Command {
    /// Initialize a project.
    Init(init::Command),

    /// List the available devices.
    Devices(devices::Command),

    /// Build a project.
    #[clap(visible_alias = "b")]
    Build(build::Command),

    /// Run a project.
    #[clap(visible_alias = "r")]
    Run(run::Command),
}
