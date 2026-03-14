use std::fs;

use cargo_metadata::MetadataCommand;
use clap::Parser;
use eyre::OptionExt;

#[derive(Parser)]
pub struct Command {
    #[clap(subcommand)]
    system: System,
}

#[derive(Parser)]
enum System {
    /// Initialize project files for android.
    Android(Android),
}

#[derive(Parser)]
struct Android {}

impl Command {
    pub fn run(self) -> eyre::Result<()> {
        match self.system {
            System::Android(android) => android.run(),
        }
    }
}

impl Android {
    fn run(self) -> eyre::Result<()> {
        let metadata = MetadataCommand::new().exec()?;

        let root_package = metadata
            .root_package()
            .ok_or_eyre("root package could not be found")?;

        let root_path = root_package
            .manifest_path
            .parent()
            .expect("files always have a parent directory");

        let android_dir = root_path.join("android");
        let main_dir = android_dir.join("src").join("main");

        if android_dir.exists() {
            eyre::bail!("android project already exists");
        }

        fs::create_dir_all(&main_dir)?;

        fs::write(
            android_dir.join(".gitignore"),
            include_str!("../template/android/.gitignore"),
        )?;

        fs::write(
            android_dir.join("build.gradle.kts"),
            include_str!("../template/android/build.gradle.kts"),
        )?;

        fs::write(
            android_dir.join("gradle.properties"),
            include_str!("../template/android/gradle.properties"),
        )?;

        fs::write(
            android_dir.join("settings.gradle.kts"),
            include_str!("../template/android/settings.gradle.kts"),
        )?;

        fs::write(
            main_dir.join("AndroidManifest.xml"),
            include_str!("../template/android/src/main/AndroidManifest.xml"),
        )?;

        eprintln!("android project initialized!");

        Ok(())
    }
}
