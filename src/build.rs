use std::process;

use cargo_metadata::MetadataCommand;
use clap::Parser;
use owo_colors::OwoColorize;

use crate::metadata;

#[derive(Parser)]
pub struct Command {
    #[clap(subcommand)]
    system: System,

    #[clap(flatten)]
    settings: Settings,
}

#[derive(Parser)]
pub struct Settings {
    #[clap(short, long)]
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
            System::Android(android) => android.run(&self.settings),
        }
    }
}

impl Android {
    fn run(self, settings: &Settings) -> eyre::Result<()> {
        let meta = MetadataCommand::new().exec()?;
        let meta = metadata::Android::new(&meta)?;
        android(&meta, settings)
    }
}

pub fn android(meta: &metadata::Android, settings: &Settings) -> eyre::Result<()> {
    eprintln!(
        "   {} {} v{} (android)",
        "Building".green().bold(),
        &meta.package,
        meta.root_package.version,
    );

    let assemble_task = match settings.release {
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
