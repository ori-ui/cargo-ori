use std::{fmt, process};

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

        for device in devices {
            eprintln!("    {device}");
        }

        Ok(())
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
}

#[derive(Clone, Debug)]
pub struct Android {
    pub id: String,
    pub device: String,
}

impl Device {
    pub fn fuzzy_match(&self, matcher: &mut Matcher, needle: &str) -> Option<u16> {
        match self {
            Device::Desktop(desktop) => desktop.fuzzy_match(matcher, needle),
            Device::Android(android) => android.fuzzy_match(matcher, needle),
        }
    }
}

impl Desktop {
    pub fn fuzzy_match(&self, matcher: &mut Matcher, needle: &str) -> Option<u16> {
        matcher.fuzzy_match_all(["desktop", &self.os], needle)
    }
}

impl Android {
    pub fn fuzzy_match(&self, matcher: &mut Matcher, needle: &str) -> Option<u16> {
        matcher.fuzzy_match_all(["mobile", "android", &self.id, &self.device], needle)
    }
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Device::Desktop(desktop) => desktop.fmt(f),
            Device::Android(android) => android.fmt(f),
        }
    }
}

impl fmt::Display for Desktop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (desktop) - linux", self.os)
    }
}

impl fmt::Display for Android {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (android) - {}", self.device, self.id)
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

        let desktop = Desktop { os: os.to_owned() };

        devices.push(Device::Desktop(desktop));
    }

    if let Ok(android) = list_android(cargo) {
        devices.extend(android.into_iter().map(Device::Android));
    }

    devices
}

pub fn list_android(cargo: &Metadata) -> eyre::Result<Vec<Android>> {
    let adb = metadata::Android::get_adb(cargo)?;

    let devices_output = process::Command::new(adb)
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

        if let Ok(device) = Android::parse(parts) {
            devices.push(device);
        }
    }

    Ok(devices)
}

impl Android {
    fn parse<'a>(mut parts: impl Iterator<Item = &'a str>) -> eyre::Result<Android> {
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

        Ok(Android {
            id: id.to_owned(),
            device: device.to_owned(),
        })
    }
}
