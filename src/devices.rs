use std::{env, ffi::OsStr, process};

use cargo_metadata::{Metadata, MetadataCommand, TargetKind};
use clap::Parser;
use eyre::OptionExt;
use nucleo_matcher::Utf32Str;
use owo_colors::OwoColorize;

use crate::metadata;

#[derive(Parser)]
pub struct Command {}

impl Command {
    pub fn run(self) -> eyre::Result<()> {
        let cargo = MetadataCommand::new().exec()?;
        let devices = list(&cargo);

        eprintln!("{}", "List of available devices:".green());

        let rows = devices.iter().map(Device::table_items).collect::<Vec<_>>();
        tabularize(&rows);

        Ok(())
    }
}

fn tabularize(rows: &[Vec<String>]) {
    let mut lengths = Vec::new();

    for row in rows {
        for (i, item) in row.iter().enumerate() {
            match lengths.get_mut(i) {
                Some(length) => *length = usize::max(*length, item.len()),
                None => lengths.push(item.len()),
            }
        }
    }

    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            eprintln!();
        }

        eprint!("    ");

        for (i, (item, length)) in row.iter().zip(&lengths).enumerate() {
            if i > 1 {
                eprint!(" - ");
            } else if i > 0 {
                eprint!(" ");
            }

            eprint!("{item}{}", " ".repeat(length - item.len()));
        }
    }
}

pub struct Matcher {
    matcher: nucleo_matcher::Matcher,
    haystack: Vec<char>,
    needle: Vec<char>,
}

impl Matcher {
    pub fn new() -> Self {
        let mut config = nucleo_matcher::Config::DEFAULT;
        config.ignore_case = true;

        Self {
            matcher: nucleo_matcher::Matcher::new(config),
            haystack: Vec::new(),
            needle: Vec::new(),
        }
    }

    pub fn fuzzy_match(
        &mut self,
        haystack: impl AsRef<str>,
        needle: impl AsRef<str>,
    ) -> Option<u16> {
        let haystack = Utf32Str::new(haystack.as_ref(), &mut self.haystack);
        let needle = Utf32Str::new(needle.as_ref(), &mut self.needle);

        self.matcher.fuzzy_match(haystack, needle)
    }

    pub fn fuzzy_match_all<'a>(
        &mut self,
        haystacks: impl IntoIterator<Item = &'a str>,
        needle: impl AsRef<str>,
    ) -> Option<u16> {
        let needle = needle.as_ref();

        haystacks
            .into_iter()
            .filter_map(|haystack| self.fuzzy_match(haystack, needle))
            .max()
    }
}

#[derive(Clone, Debug)]
pub enum Device {
    Desktop(Desktop),
    Android(Android),
}

#[derive(Clone, Debug)]
pub struct Desktop {
    pub os: String,
    pub arch: String,
}

#[derive(Clone, Debug)]
pub struct Android {
    pub id: String,
    pub device: String,
    pub arch: String,
}

impl Device {
    pub fn fuzzy_match(&self, matcher: &mut Matcher, needle: &str) -> Option<u16> {
        match self {
            Device::Desktop(device) => device.fuzzy_match(matcher, needle),
            Device::Android(device) => device.fuzzy_match(matcher, needle),
        }
    }

    pub fn table_items(&self) -> Vec<String> {
        match self {
            Device::Desktop(device) => device.table_items(),
            Device::Android(device) => device.table_items(),
        }
    }
}

impl Desktop {
    pub fn fuzzy_match(&self, matcher: &mut Matcher, needle: &str) -> Option<u16> {
        matcher.fuzzy_match_all(["desktop", &self.os], needle)
    }

    pub fn table_items(&self) -> Vec<String> {
        vec![
            self.os.clone(),
            String::from("(desktop)"),
            self.os.clone(),
            self.arch.clone(),
        ]
    }
}

impl Android {
    pub fn fuzzy_match(&self, matcher: &mut Matcher, needle: &str) -> Option<u16> {
        matcher.fuzzy_match_all(["mobile", "android", &self.id, &self.device], needle)
    }

    pub fn table_items(&self) -> Vec<String> {
        vec![
            self.device.clone(),
            String::from("(android)"),
            self.id.clone(),
            self.arch.clone(),
        ]
    }
}

pub fn list(cargo: &Metadata) -> Vec<Device> {
    let mut devices = Vec::new();

    if let Some(root_package) = cargo.root_package()
        && root_package
            .targets
            .iter()
            .any(|target| target.kind.contains(&TargetKind::Bin))
    {
        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };

        let desktop = Desktop {
            os: os.to_owned(),
            arch: env::consts::ARCH.to_owned(),
        };

        devices.push(Device::Desktop(desktop));
    }

    if let Ok(android) = list_android(cargo) {
        devices.extend(android.into_iter().map(Device::Android));
    }

    devices
}

pub fn list_android(cargo: &Metadata) -> eyre::Result<Vec<Android>> {
    let adb = metadata::Android::get_adb(cargo)?;

    let devices_output = process::Command::new(&adb)
        .arg("devices")
        .arg("-l")
        .output()?;

    if !devices_output.status.success() {
        eyre::bail!("getting android devices failed");
    }

    let stdout = String::from_utf8_lossy(&devices_output.stdout);

    let mut devices = Vec::new();
    for line in stdout.lines().skip(1) {
        let parts = line.split_whitespace();

        if let Ok(device) = Android::parse(&adb, parts) {
            devices.push(device);
        }
    }

    Ok(devices)
}

impl Android {
    fn parse<'a>(adb: &OsStr, mut parts: impl Iterator<Item = &'a str>) -> eyre::Result<Android> {
        let id = parts.next().ok_or_eyre("id not found")?;

        parts.next(); // device
        parts.next(); // usb
        parts.next(); // product
        parts.next(); // model

        let device = parts.next().ok_or_eyre("device not found")?;
        let device = device
            .split(':')
            .next_back()
            .ok_or_eyre("device not found")?;

        let arch = process::Command::new(adb)
            .arg("-s")
            .arg(id)
            .arg("shell")
            .arg("uname")
            .arg("-m")
            .output()?;

        Ok(Android {
            id: id.to_owned(),
            device: device.to_owned(),
            arch: String::from_utf8_lossy(&arch.stdout).into(),
        })
    }
}
