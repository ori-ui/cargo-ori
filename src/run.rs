use std::{
    env,
    io::{self, prelude::*},
    process,
};

use cargo_metadata::MetadataCommand;
use clap::Parser;
use owo_colors::OwoColorize;

use crate::{build, metadata};

#[derive(Parser)]
pub struct Command {
    #[clap(subcommand)]
    system: System,

    #[clap(flatten)]
    settings: build::Settings,
}

#[derive(Parser)]
enum System {
    /// Run project for android.
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
    fn run(self, settings: &build::Settings) -> eyre::Result<()> {
        let meta = MetadataCommand::new().exec()?;
        let meta = metadata::Android::new(&meta)?;

        build::android(&meta, settings)?;

        let adb_output = process::Command::new(&meta.gradle)
            .current_dir(&meta.android_directory)
            .arg("getAdbExe")
            .arg("--quiet")
            .output();

        let adb = if let Ok(output) = adb_output
            && output.status.success()
        {
            String::from_utf8_lossy(&output.stdout).trim().into()
        } else if let Ok(sdk_root) = env::var("ANDROID_SDK_ROOT") {
            if cfg!(target_os = "windows") {
                format!("{sdk_root}/platform-tools/adb.exe")
            } else {
                format!("{sdk_root}/platform-tools/adb")
            }
        } else {
            String::from("adb")
        };

        let apk = match settings.release {
            true => {
                let release = meta.android_directory.join("build/outputs/apk/release");
                let signed = release.join("android-release.apk");

                if signed.exists() {
                    signed
                } else {
                    release.join("android-release-unsigned.apk")
                }
            }

            false => meta
                .android_directory
                .join("build/outputs/apk/debug/android-debug.apk"),
        };

        if !apk.exists() {
            eyre::bail!("apk file does not exist");
        }

        let stop_output = process::Command::new(&adb)
            .arg("shell")
            .arg("am")
            .arg("force-stop")
            .arg(&meta.package)
            .output()?;

        let mut stdout = io::stdout();

        if !stop_output.status.success() {
            stdout.write_all(&stop_output.stderr)?;
        }

        let install_output = process::Command::new(&adb)
            .arg("install")
            .arg("-t")
            .arg("-r")
            .arg(apk)
            .output()?;

        if !install_output.status.success() {
            stdout.write_all(&install_output.stderr)?;
            eyre::bail!("installing app failed");
        }

        process::Command::new(&adb)
            .arg("logcat")
            .arg("-c")
            .output()?;

        eprintln!(
            "     {} `{}/ori.OriActivity`",
            "Running".green().bold(),
            meta.package,
        );

        let start_output = process::Command::new(&adb)
            .arg("shell")
            .arg("am")
            .arg("start-activity")
            .arg("-n")
            .arg(&meta.activity)
            .arg("-W")
            .output()?;

        if !start_output.status.success() {
            stdout.write_all(&start_output.stderr)?;
            eyre::bail!("starting app failed");
        }

        let pid_output = process::Command::new(&adb)
            .arg("shell")
            .arg("pidof")
            .arg("-s")
            .arg(&meta.package)
            .output()?;

        if !pid_output.status.success() {
            stdout.write_all(&pid_output.stderr)?;
            eyre::bail!("could not get `pid` of `{}`", meta.package);
        }

        let pid = String::from_utf8_lossy(&pid_output.stdout);

        process::Command::new(&adb)
            .arg("logcat")
            .arg("-s")
            .arg("rust")
            .arg(format!("--pid={}", pid.trim()))
            .arg("-v")
            .arg("raw")
            .stdout(process::Stdio::inherit())
            .spawn()?
            .wait()?;

        Ok(())
    }
}
