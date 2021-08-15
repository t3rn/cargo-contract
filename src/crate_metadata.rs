// Copyright 2018-2020 Parity Technologies (UK) Ltd.
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

use crate::workspace::ManifestPath;
use anyhow::{Context, Result};
use cargo_metadata::{Metadata as CargoMetadata, MetadataCommand, Package};
use colored::Colorize;
use semver::Version;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{fs, path::PathBuf};
use toml::value;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct ComposableDeployConfig {
    pub compose: String,
    pub vm: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ComposableExecConfig {
    pub compose: String,
    pub gateway: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ComposableScheduleMetadata {
    pub composables: Vec<String>,
    pub schedule: Option<String>,
    pub deploy: Option<Vec<ComposableDeployConfig>>,
    pub exec: Option<Vec<ComposableExecConfig>>,
}

/// Relevant metadata obtained from Cargo.toml.
#[derive(Debug, Clone)]
pub struct CrateMetadata {
    pub manifest_path: ManifestPath,
    pub cargo_meta: cargo_metadata::Metadata,
    pub package_name: String,
    pub t3rn_composable_schedule: Option<ComposableScheduleMetadata>,
    pub root_package: Package,
    pub original_wasm: PathBuf,
    pub target_directory: PathBuf,
    pub dest_wasm: PathBuf,
    pub ink_version: Version,
    pub documentation: Option<Url>,
    pub homepage: Option<Url>,
    pub user: Option<Map<String, Value>>,
}

impl CrateMetadata {
    /// Parses the contract manifest and returns relevant metadata.
    pub fn collect(manifest_path: &ManifestPath) -> Result<Self> {
        let (metadata, root_package) = get_cargo_metadata(manifest_path)?;

        // Normalize the package name.
        let package_name = root_package.name.replace("-", "_");

        // {target_dir}/wasm32-unknown-unknown/release/{package_name}.wasm
        let mut original_wasm = metadata.target_directory.clone();
        original_wasm.push("wasm32-unknown-unknown");
        original_wasm.push("release");
        original_wasm.push(package_name.clone());
        original_wasm.set_extension("wasm");

        // {target_dir}/{package_name}.wasm
        let mut dest_wasm = metadata.target_directory.clone();
        dest_wasm.push(package_name.clone());
        dest_wasm.set_extension("wasm");

        let mut composable_schedule: Option<ComposableScheduleMetadata> = None;

        // let user = toml
        //     .get("package")
        //     .and_then(|v| v.get("metadata"))
        //     .and_then(|v| v.get("contract"))
        //     .and_then(|v| v.get("user"))
        //     .and_then(|v| v.as_table())
        //     .map(|v| {
        //         // convert user defined section from toml to json
        //         serde_json::to_string(v).and_then(|json| serde_json::from_str(&json))
        //     })
        //     .transpose()?;

        let ink_version = metadata
            .packages
            .iter()
            .find_map(|package| {
                // Extract the composable metadata which should be place in the Cargo.toml of package_name.
                if package.name == package_name.clone() {
                    composable_schedule = match serde_json::from_value(package.metadata.clone()) {
                        Ok(composable_schedule) => Some(composable_schedule),
                        Err(_) => None,
                    };
                    if let Some(t3rn_schedule) = composable_schedule.clone() {
                        println!(
                            "{} {:?}",
                            "Detected t3rn schedule with following components:"
                                .bright_blue()
                                .bold(),
                            t3rn_schedule.composables
                        );
                    }
                }
                if package.name == "ink_lang" {
                    Some(
                        Version::parse(&package.version.to_string())
                            .expect("Invalid ink_lang version string"),
                    )
                } else {
                    None
                }
            })
            .ok_or(anyhow::anyhow!("No 'ink_lang' dependency found"))?;

        let (documentation, homepage, user) = get_cargo_toml_metadata(manifest_path)?;

        let crate_metadata = CrateMetadata {
            manifest_path: manifest_path.clone(),
            cargo_meta: metadata.clone(),
            root_package,
            package_name,
            original_wasm,
            dest_wasm,
            ink_version,
            documentation,
            homepage,
            user,
            t3rn_composable_schedule: composable_schedule,
            target_directory: metadata.target_directory.clone(),
        };
        Ok(crate_metadata)
    }
}

/// Get the result of `cargo metadata`, together with the root package id.
fn get_cargo_metadata(manifest_path: &ManifestPath) -> Result<(CargoMetadata, Package)> {
    let mut cmd = MetadataCommand::new();
    let metadata = cmd
        .manifest_path(manifest_path)
        .exec()
        .context("Error invoking `cargo metadata`")?;
    let root_package_id = metadata
        .resolve
        .as_ref()
        .and_then(|resolve| resolve.root.as_ref())
        .context("Cannot infer the root project id")?
        .clone();
    // Find the root package by id in the list of packages. It is logical error if the root
    // package is not found in the list.
    let root_package = metadata
        .packages
        .iter()
        .find(|package| package.id == root_package_id)
        .expect("The package is not found in the `cargo metadata` output")
        .clone();
    Ok((metadata, root_package))
}

/// Read extra metadata not available via `cargo metadata` directly from `Cargo.toml`
fn get_cargo_toml_metadata(
    manifest_path: &ManifestPath,
) -> Result<(Option<Url>, Option<Url>, Option<Map<String, Value>>)> {
    let toml = fs::read_to_string(manifest_path)?;
    let toml: value::Table = toml::from_str(&toml)?;

    let get_url = |field_name| -> Result<Option<Url>> {
        toml.get("package")
            .ok_or(anyhow::anyhow!("package section not found"))?
            .get(field_name)
            .and_then(|v| v.as_str())
            .map(Url::parse)
            .transpose()
            .context(format!("{} should be a valid URL", field_name))
            .map_err(Into::into)
    };

    let documentation = get_url("documentation")?;
    let homepage = get_url("homepage")?;

    let user = toml
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("contract"))
        .and_then(|v| v.get("user"))
        .and_then(|v| v.as_table())
        .map(|v| {
            // convert user defined section from toml to json
            serde_json::to_string(v).and_then(|json| serde_json::from_str(&json))
        })
        .transpose()?;

    Ok((documentation, homepage, user))
}
