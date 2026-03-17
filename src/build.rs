use std::process;

use cargo_metadata::MetadataCommand;
use clap::Parser;

use crate::metadata;

#[derive(Parser)]
pub struct Command {
    #[clap(subcommand)]
    system: System,

    #[clap(flatten)]
    options: Options,
}

#[derive(Parser)]
pub struct Options {
    /// Package with the target to run.
    #[arg(short, long, help_heading = "Package Selection")]
    pub package: Option<String>,

    /// Name of the bin target to build.
    #[arg(long, help_heading = "Target Selection")]
    pub bin: Option<String>,

    /// Build application in release mode, with optimizations.
    #[arg(short, long, help_heading = "Compilation Options")]
    pub release: bool,
}

#[derive(Parser)]
enum System {
    /// Build project for android.
    #[clap(visible_alias = "a")]
    Android(Android),
}

#[derive(Parser)]
struct Android {}

impl Command {
    pub fn run(self) -> eyre::Result<()> {
        match self.system {
            System::Android(android) => android.run(&self.options),
        }
    }
}

impl Android {
    fn run(self, options: &Options) -> eyre::Result<()> {
        let meta = MetadataCommand::new().exec()?;
        let meta = metadata::Android::new(&meta, options.package.as_deref())?;

        android(&meta, options.release)
    }
}

pub fn android(meta: &metadata::Android, release: bool) -> eyre::Result<()> {
    let assemble_task = match release {
        true => "assembleRelease",
        false => "assembleDebug",
    };

    let build_exit_code = process::Command::new(&meta.gradle)
        .current_dir(&meta.android_directory)
        .arg(assemble_task)
        .arg("--quiet")
        .spawn()?
        .wait()?;

    if !build_exit_code.success() {
        eyre::bail!("gradle build failed");
    }

    Ok(())
}
