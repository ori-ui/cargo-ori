use std::{
    io::{self, prelude::*},
    process,
};

use cargo_metadata::{Metadata, MetadataCommand};
use clap::Parser;
use owo_colors::OwoColorize;

use crate::{build, devices, metadata};

#[derive(Parser)]
pub struct Command {
    /// The device to run the application on.
    device: Option<String>,

    #[clap(flatten)]
    options: Options,
}

#[derive(Parser)]
struct Options {
    /// Package with the target to run.
    #[arg(short, long, help_heading = "Package Selection")]
    package: Option<String>,

    /// Name of the bin target to run.
    #[arg(long, help_heading = "Target Selection")]
    bin: Option<String>,

    /// Name of the example target to run.
    #[arg(long, help_heading = "Target Selection")]
    example: Option<String>,

    /// Build application in release mode, with optimizations.
    #[arg(short, long, help_heading = "Compilation Options")]
    release: bool,
}

impl Command {
    pub fn run(self) -> eyre::Result<()> {
        let cargo = MetadataCommand::new().exec()?;
        let mut matcher = devices::Matcher::new();

        let device = if let Some(needle) = self.device {
            devices::list(&cargo)
                .into_iter()
                .filter_map(|device| {
                    let score = device.fuzzy_match(&mut matcher, &needle)?;
                    Some((score, device))
                })
                .max_by_key(|(score, _)| *score)
                .map(|(_, device)| device)
        } else {
            devices::list(&cargo).pop()
        };

        let Some(device) = device else {
            eyre::bail!("no device found");
        };

        match device {
            devices::Device::Desktop(device) => desktop(&cargo, &device, &self.options),
            devices::Device::Android(device) => android(&cargo, &device, &self.options),
        }
    }
}

fn desktop(_cargo: &Metadata, _device: &devices::Desktop, options: &Options) -> eyre::Result<()> {
    let mut command = process::Command::new("cargo");

    command.arg("run");

    if options.release {
        command.arg("--release");
    }

    if let Some(ref bin) = options.bin {
        command.arg("--bin").arg(bin);
    }

    if let Some(ref example) = options.example {
        command.arg("--example").arg(example);
    }

    if let Some(ref package) = options.package {
        command.arg("--package").arg(package);
    }

    command.spawn()?.wait()?;

    Ok(())
}

fn android(cargo: &Metadata, device: &devices::Android, options: &Options) -> eyre::Result<()> {
    let meta = metadata::Android::new(cargo, options.package.as_deref())?;

    build::android(&meta, options.release)?;

    let apk = match options.release {
        true => {
            let release = meta.android_directory.join("build/outputs/apk/release");

            if release.join("android-release-unsigned.apk").exists()
                && !release.join("android-release.apk").exists()
            {
                eyre::bail!("only signed release builds can be run");
            }

            release.join("android-release.apk")
        }

        false => meta
            .android_directory
            .join("build/outputs/apk/debug/android-debug.apk"),
    };

    if !apk.exists() {
        eyre::bail!("apk file does not exist");
    }

    let adb = metadata::Android::get_adb(cargo)?;

    let stop_output = process::Command::new(&adb)
        .arg("-s")
        .arg(&device.id)
        .arg("shell")
        .arg("am")
        .arg("force-stop")
        .arg(&meta.app_id)
        .output()?;

    let mut stdout = io::stdout();

    if !stop_output.status.success() {
        stdout.write_all(&stop_output.stderr)?;
    }

    let install_output = process::Command::new(&adb)
        .arg("-s")
        .arg(&device.id)
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
        .arg("-s")
        .arg(&device.id)
        .arg("logcat")
        .arg("-c")
        .output()?;

    eprintln!(
        "     {} {} ({}:{})",
        "Running".green().bold(),
        meta.app_id,
        device.device,
        device.id
    );

    let start_output = process::Command::new(&adb)
        .arg("-s")
        .arg(&device.id)
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
        .arg("-s")
        .arg(&device.id)
        .arg("shell")
        .arg("pidof")
        .arg("-s")
        .arg(&meta.app_id)
        .output()?;

    if !pid_output.status.success() {
        stdout.write_all(&pid_output.stderr)?;
        eyre::bail!("could not get `pid` of `{}`", meta.app_id);
    }

    let pid = String::from_utf8_lossy(&pid_output.stdout);

    process::Command::new(&adb)
        .arg("-s")
        .arg(&device.id)
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
