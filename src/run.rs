use std::{
    ffi::OsString,
    io::{self, Write},
    process::{self, Stdio},
};

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
    /// Runs android project.
    #[clap(visible_alias = "a")]
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

        let root_dir = root_package
            .manifest_path
            .parent()
            .expect("files always have a parent directory");

        let android_dir = root_dir.join("android");

        let android_metadata = root_package
            .metadata
            .get("android")
            .ok_or_eyre("could not find `package.metadata.android`")?;

        check_android_metadata(android_metadata);

        let package = android_metadata
            .get("package")
            .ok_or_eyre("could not find `package.metadata.android.package`")?
            .as_str()
            .ok_or_eyre("`android.package` must be a string")?;

        let activity = format!("{package}/ori.OriActivity");

        if !android_dir.exists() {
            eyre::bail!(
                "package `{}` does not have an android project",
                root_package.name,
            );
        }

        let gradlew_path = if cfg!(target_os = "windows") {
            android_dir.join("gradlew.bat")
        } else {
            android_dir.join("gradlew")
        };

        let gradle = if gradlew_path.exists() {
            gradlew_path.as_os_str().to_owned()
        } else {
            OsString::from("gradle")
        };

        let install_output = process::Command::new(&gradle)
            .current_dir(android_dir)
            .arg("installDebug")
            .stderr(Stdio::inherit())
            .output()?;

        let mut stdout = io::stdout();

        if !install_output.status.success() {
            stdout.write_all(&install_output.stderr)?;
            eyre::bail!("gradle build failed");
        }

        let adb_output = process::Command::new(&gradle)
            .arg("getAdbExe")
            .arg("--quiet")
            .output();

        let adb = if let Ok(output) = adb_output
            && output.status.success()
        {
            String::from_utf8_lossy(&output.stdout).into()
        } else {
            String::from("adb")
        };

        process::Command::new(&adb)
            .arg("logcat")
            .arg("-c")
            .output()?;

        let start_output = process::Command::new(&adb)
            .arg("shell")
            .arg("am")
            .arg("start-activity")
            .arg("-n")
            .arg(activity)
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
            .arg(package)
            .output()?;

        if !pid_output.status.success() {
            stdout.write_all(&pid_output.stderr)?;
            eyre::bail!("could not get `pid` of `{package}`");
        }

        let pid = String::from_utf8_lossy(&pid_output.stdout);

        process::Command::new(&adb)
            .arg("logcat")
            .arg("-s")
            .arg("rust")
            .arg(format!("--pid={}", pid.trim()))
            .arg("-v")
            .arg("raw")
            .stdout(Stdio::inherit())
            .spawn()?
            .wait()?;

        Ok(())
    }
}

fn check_android_metadata(metadata: &serde_json::Value) {
    const VALID_KEYS: &[&str] = &[
        "name",
        "key-properties",
        "package",
        "compile-sdk",
        "target-sdk",
        "min-sdk",
        "version",
        "version-code",
    ];

    let Some(object) = metadata.as_object() else {
        return;
    };

    for key in object.keys() {
        if !VALID_KEYS.contains(&key.as_str()) {
            eprintln!("unused key `package.metadata.android.{key}`");
        }
    }
}
