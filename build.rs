// Copyright 2018-2021 Parity Technologies (UK) Ltd.
// This file is part of cargo-contract.
//
// cargo-contract is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// cargo-contract is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with cargo-contract.  If not, see <http://www.gnu.org/licenses/>.

use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    fs::File,
    io::{prelude::*, Write},
    iter::Iterator,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;
use walkdir::WalkDir;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use platforms::{TARGET_ARCH, TARGET_ENV, TARGET_OS};
use substrate_build_script_utils::rerun_if_git_head_changed;

const DEFAULT_UNIX_PERMISSIONS: u32 = 0o755;

fn main() {
    let manifest_dir: PathBuf = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo")
        .into();
    let out_dir: PathBuf = env::var("OUT_DIR")
        .expect("OUT_DIR should be set by cargo")
        .into();

    let template_dir = manifest_dir.join("templates").join("new");
    let dst_file = out_dir.join("template.zip");

    println!(
        "Creating template zip: template_dir '{}', destination archive '{}'",
        template_dir.display(),
        dst_file.display()
    );

    generate_cargo_keys();
    rerun_if_git_head_changed();

    std::process::exit(
        match zip_dir(&template_dir, &dst_file, CompressionMethod::Stored) {
            Ok(_) => {
                println!(
                    "done: {} written to {}",
                    template_dir.display(),
                    dst_file.display()
                );
                0
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                1
            }
        },
    );
}

fn zip_dir(src_dir: &Path, dst_file: &Path, method: CompressionMethod) -> Result<()> {
    if !src_dir.exists() {
        anyhow::bail!("src_dir '{}' does not exist", src_dir.display());
    }
    if !src_dir.is_dir() {
        anyhow::bail!("src_dir '{}' is not a directory", src_dir.display());
    }

    let file = File::create(dst_file)?;

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter().filter_map(|e| e.ok());

    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(method)
        .unix_permissions(DEFAULT_UNIX_PERMISSIONS);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let mut name = path.strip_prefix(&src_dir)?.to_path_buf();

        // Cargo.toml files cause the folder to excluded from `cargo package` so need to be renamed
        if name.file_name() == Some(OsStr::new("_Cargo.toml")) {
            name.set_file_name("Cargo.toml");
        }

        let file_path = name.as_os_str().to_string_lossy();

        if path.is_file() {
            zip.start_file(file_path, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(file_path, options)?;
        }
    }
    zip.finish()?;

    Ok(())
}

/// Generate the `cargo:` key output
fn generate_cargo_keys() {
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output();

    let commit = match output {
        Ok(o) if o.status.success() => {
            let sha = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            Cow::from(sha)
        }
        Ok(o) => {
            println!("cargo:warning=Git command failed with status: {}", o.status);
            Cow::from("unknown")
        }
        Err(err) => {
            println!("cargo:warning=Failed to execute git command: {}", err);
            Cow::from("unknown")
        }
    };

    println!(
        "cargo:rustc-env=CARGO_CONTRACT_CLI_IMPL_VERSION={}",
        get_version(&commit)
    )
}

fn get_version(impl_commit: &str) -> String {
    let commit_dash = if impl_commit.is_empty() { "" } else { "-" };

    format!(
        "{}{}{}-{}",
        std::env::var("CARGO_PKG_VERSION").unwrap_or_default(),
        commit_dash,
        impl_commit,
        get_platform(),
    )
}

fn get_platform() -> String {
    let env_dash = if TARGET_ENV.is_some() { "-" } else { "" };

    format!(
        "{}-{}{}{}",
        TARGET_ARCH.as_str(),
        TARGET_OS.as_str(),
        env_dash,
        TARGET_ENV.map(|x| x.as_str()).unwrap_or(""),
    )
}
