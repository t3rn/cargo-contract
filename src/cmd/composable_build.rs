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

use crate::{
    crate_metadata::CrateMetadata,
    util,
    workspace::{ManifestPath, Profile, Workspace},
    UnstableFlags, Verbosity,
};
use anyhow::{Context, Result};
use colored::Colorize;
use parity_wasm::elements::{External, MemoryType, Module, Section};
use regex::Regex;
use std::io::prelude::*;
use std::{
    env, fs,
    fs::metadata,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};
use wat;

/// This is the maximum number of pages available for a contract to allocate.
const MAX_MEMORY_PAGES: u32 = 16;

// &format!("--target-dir={}", target_dir.to_string_lossy() + "/" + current_compose_name),
/// Constructs a target destination path for a current compose.
/// This is simply creating additional folder with a compose name in a target directory.
pub fn get_compose_target_dest(compose: String, mut target_dir: PathBuf) -> PathBuf {
    target_dir.push(compose);
    target_dir
}

/// Constructs a path to the original (before optimisations) WASM file
pub fn get_original_wasm_path(compose: String, crate_metadata: &CrateMetadata) -> PathBuf {
    let mut original_wasm =
        get_compose_target_dest(compose, crate_metadata.target_directory.clone()).clone();
    // let mut original_wasm = metadata.target_directory.clone();
    original_wasm.push("wasm32-unknown-unknown");
    original_wasm.push("release");
    original_wasm.push(crate_metadata.package_name.clone());
    original_wasm.set_extension("wasm");
    original_wasm
}

/// Constructs a path to the destination (after optimisations) WASM file
pub fn get_dest_wasm_path(compose: String, crate_metadata: &CrateMetadata) -> PathBuf {
    let mut dest_wasm =
        get_compose_target_dest(compose.clone(), crate_metadata.target_directory.clone()).clone();
    // let mut dest_wasm = metadata.target_directory.clone();
    dest_wasm.push(compose.clone());
    dest_wasm.set_extension("wasm");
    dest_wasm
}

/// Builds the project in the specified directory, defaults to the current directory.
///
/// Uses [`cargo-xbuild`](https://github.com/rust-osdev/cargo-xbuild) for maximum optimization of
/// the resulting Wasm binary.
///
/// # Cargo.toml optimizations
///
/// The original Cargo.toml will be amended to remove the `rlib` crate type in order to minimize
/// the final Wasm binary size.
///
/// Preferred default `[profile.release]` settings will be added if they are missing, existing
/// user-defined settings will be preserved.
///
/// To disable this and use the original `Cargo.toml` as is then pass the `-Z original_manifest` flag.
fn build_cargo_project_compose(
    crate_metadata: &CrateMetadata,
    current_compose_name: String,
    verbosity: Option<Verbosity>,
    unstable_options: UnstableFlags,
) -> Result<()> {
    util::assert_channel()?;

    // set RUSTFLAGS, read from environment var by cargo-xbuild
    std::env::set_var(
        "RUSTFLAGS",
        "-C link-arg=-z -C link-arg=stack-size=65536 -C link-arg=--import-memory",
    );

    let verbosity = verbosity.map(|v| match v {
        Verbosity::Verbose => xargo_lib::Verbosity::Verbose,
        Verbosity::Quiet => xargo_lib::Verbosity::Quiet,
    });

    let xbuild = |manifest_path: &ManifestPath| {
        let manifest_path = Some(manifest_path);
        let target = Some("wasm32-unknown-unknown");
        let target_dir = &crate_metadata.cargo_meta.target_directory;

        let other_args = [
            "--no-default-features",
            &format!("--features={}", current_compose_name),
            "--release",
            &format!(
                "--target-dir={}",
                get_compose_target_dest(current_compose_name, target_dir.to_path_buf())
                    .to_string_lossy()
            ),
        ];
        let args = xargo_lib::Args::new(target, manifest_path, verbosity, &other_args)
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Creating xargo args")?;

        let config = xargo_lib::Config {
            sysroot_path: target_dir.join("sysroot"),
            memcpy: false,
            panic_immediate_abort: true,
        };
        let exit_status = xargo_lib::build(args, "build", Some(config))
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Building with xbuild")?;
        if !exit_status.success() {
            anyhow::bail!("xbuild failed with status {}", exit_status)
        }
        Ok(())
    };

    if unstable_options.original_manifest {
        println!(
            "{} {}",
            "warning:".yellow().bold(),
            "with 'original-manifest' enabled, the contract binary may not be of optimal size."
                .bold()
        );
        xbuild(&crate_metadata.manifest_path)?;
    } else {
        Workspace::new(&crate_metadata.cargo_meta, &crate_metadata.root_package.id)?
            .with_root_package_manifest(|manifest| {
                manifest
                    .with_removed_crate_type("rlib")?
                    .with_profile_release_defaults(Profile::default_contract_release())?;
                Ok(())
            })?
            .using_temp(xbuild)?;
    }

    // clear RUSTFLAGS
    std::env::remove_var("RUSTFLAGS");

    Ok(())
}

/// Ensures the wasm memory import of a given module has the maximum number of pages.
///
/// Iterates over the import section, finds the memory import entry if any and adjusts the maximum
/// limit.
fn ensure_maximum_memory_pages(module: &mut Module, maximum_allowed_pages: u32) -> Result<()> {
    let mem_ty = module
        .import_section_mut()
        .and_then(|section| {
            section
                .entries_mut()
                .iter_mut()
                .find_map(|entry| match entry.external_mut() {
                    External::Memory(ref mut mem_ty) => Some(mem_ty),
                    _ => None,
                })
        })
        .context("Memory import is not found. Is --import-memory specified in the linker args")?;

    if let Some(requested_maximum) = mem_ty.limits().maximum() {
        // The module already has maximum, check if it is within the limit bail out.
        if requested_maximum > maximum_allowed_pages {
            anyhow::bail!(
                "The wasm module requires {} pages. The maximum allowed number of pages is {}",
                requested_maximum,
                maximum_allowed_pages,
            );
        }
    } else {
        let initial = mem_ty.limits().initial();
        *mem_ty = MemoryType::new(initial, Some(MAX_MEMORY_PAGES));
    }

    Ok(())
}

/// Strips all custom sections.
///
/// Presently all custom sections are not required so they can be stripped safely.
fn strip_custom_sections(module: &mut Module) {
    module.sections_mut().retain(|section| match section {
        Section::Custom(_) => false,
        Section::Name(_) => false,
        Section::Reloc(_) => false,
        _ => true,
    });
}

/// Performs required post-processing steps on the wasm artifact.
fn post_process_wasm_compose(crate_metadata: &CrateMetadata, compose: String) -> Result<()> {
    // Deserialize wasm module from a file.
    let mut module =
        parity_wasm::deserialize_file(get_original_wasm_path(compose.clone(), crate_metadata))
            .context(format!(
                "Loading original wasm file '{}'",
                crate_metadata.original_wasm.display()
            ))?;

    // Perform optimization.
    //
    // In practice only tree-shaking is performed, i.e transitively removing all symbols that are
    // NOT used by the specified entrypoints.
    if pwasm_utils::optimize(&mut module, ["call", "deploy"].to_vec()).is_err() {
        anyhow::bail!("Optimizer failed");
    }
    ensure_maximum_memory_pages(&mut module, MAX_MEMORY_PAGES)?;
    strip_custom_sections(&mut module);

    parity_wasm::serialize_to_file(get_dest_wasm_path(compose.clone(), crate_metadata), module)?;
    Ok(())
}

/// Attempts to perform optional wasm optimization using `wasm-opt`.
///
/// The intention is to reduce the size of bloated wasm binaries as a result of missing
/// optimizations (or bugs?) between Rust and Wasm.
///
/// This step depends on the `wasm-opt` tool being installed. If it is not the build will still
/// succeed, and the user will be encouraged to install it for further optimizations.
fn optimize_wasm_compose(crate_metadata: &CrateMetadata, compose: String) -> Result<()> {
    // check `wasm-opt` installed
    if which::which("wasm-opt").is_err() {
        println!(
            "{}",
            "wasm-opt is not installed. Install this tool on your system in order to \n\
             reduce the size of your contract's Wasm binary. \n\
             See https://github.com/WebAssembly/binaryen#tools"
                .bright_yellow()
        );
        return Ok(());
    }
    let dest_wasm = get_dest_wasm_path(compose.clone(), crate_metadata);
    let mut optimized = dest_wasm.clone();
    optimized.set_file_name(format!("{}-opt.wasm", compose.clone()));

    let output = Command::new("wasm-opt")
        .arg(dest_wasm.clone().as_os_str())
        .arg("-O3") // execute -O3 optimization passes (spends potentially a lot of time optimizing)
        .arg("-o")
        .arg(optimized.as_os_str())
        .output()?;

    if !output.status.success() {
        // Dump the output streams produced by wasm-opt into the stdout/stderr.
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        anyhow::bail!("wasm-opt optimization failed");
    }

    let original_size = metadata(&dest_wasm)?.len() as f64 / 1000.0;
    let optimized_size = metadata(&optimized)?.len() as f64 / 1000.0;
    println!(
        " Original wasm size: {:.1}K, Optimized: {:.1}K",
        original_size, optimized_size
    );

    // overwrite existing destination wasm file with the optimised version
    std::fs::rename(&optimized, &dest_wasm)?;
    Ok(())
}

/// Scanner for Wasm text format contracts.
/// Developers can write their contract directly in WAT format by following the following rules:
/// Assign contract as a string constant. Name of the contract must specifically match the composable name (either upper or lowe case).
/// Contract name and code are looked up by a regexp, therefore a contract must be placed always between: `r#` and `"#;`
/// static EXAMPLE_WAT_CONTRACT: &str = r#"
///         (module
///            (func (export "call"))
///            (func (export "deploy"))
///         )
///         "#;
/// Out of the above example the scanner will recognize an entry: WatContract { name: "example", code: "(module (func..."deploy")) )" }
#[derive(Debug)]
pub struct WatContractsScanner {
    contracts: Vec<WatContract>,
}

/// Entry of a smart contract as Wasm text format.
#[derive(Debug, Clone)]
pub struct WatContract {
    code: &'static str,
    name: &'static str,
}

impl WatContractsScanner {
    fn new_empty() -> WatContractsScanner {
        WatContractsScanner { contracts: vec![] }
    }

    fn scan_from_text(&mut self, text: &'static str) -> bool {
        for caps in Regex::new(
            r"(?P<compose_name>[\w]+)_WAT_CONTRACT: &str = r#(?P<wat_contract>[\s\S]+)#;",
        )
        .unwrap()
        .captures_iter(text)
        {
            self.contracts.push(WatContract {
                name: caps
                    .name("compose_name")
                    .map(|r| Box::leak(r.as_str().to_lowercase().into_boxed_str()))
                    .unwrap(),
                code: caps.name("wat_contract").map(|r| r.as_str()).unwrap(),
            });
        }
        !self.contracts.is_empty()
    }

    fn add_contract(&mut self, name: &'static str, code: &'static str) {
        self.contracts.push({ WatContract { name, code } });
    }

    fn find_by_name(&mut self, name: String) -> Option<&WatContract> {
        let r = self.contracts.iter().find(|&c| c.name == name);
        // println!("find_by_name res {:?} {:?}", name, r);
        r
    }
}
/// Load a given wasm module read from a contract file and save a compiled wasm byte code as a file.
///
/// File will be stored at the same destination path as it would be after regular compilation with xbuild.
fn compile_wat_to_wasm(
    compose_name: String,
    crate_metadata: &CrateMetadata,
    wat_contract: &WatContract,
) -> Result<()> {
    let mut dest_wat_path = get_dest_wasm_path(compose_name.clone(), crate_metadata);
    dest_wat_path.set_extension("wat");
    let mut file = File::create(dest_wat_path.clone()).map_err(|e| {
        println!(
            "{} {} {}",
            "Error when creating an empty file for WAT contract at ./target for component: "
                .bright_red()
                .bold(),
            compose_name,
            e.to_string()
        );
        e
    })?;
    let mut code_bytes = wat_contract.code.as_bytes();
    // During regexp search there are additional \" at the beginning and EOF. Strip them out as they're fail the compilation.
    file.write_all(&code_bytes[1..code_bytes.len() - 1])
        .map_err(|e| {
            println!(
                "{} {} {}",
                "Error when saving WAT contract as file at ./target for component: "
                    .bright_red()
                    .bold(),
                compose_name,
                e.to_string()
            );
            e
        })?;

    // After the .wat file is saved, read it again with wat2wasm compiler / parser
    let wasm_bytes = wat::parse_file(dest_wat_path.clone()).map_err(|e| {
        println!(
            "{} {} {}",
            "Error during WAT contract compilation for component: "
                .bright_red()
                .bold(),
            compose_name,
            e.to_string()
        );
        e
    })?;

    let mut file = File::create(get_dest_wasm_path(compose_name.clone(), crate_metadata))?;
    // Write a slice of WASM bytes to the file
    file.write_all(&wasm_bytes).map_err(|e| {
        println!(
            "{} {} {}",
            "Error when saving WASM contract as file at ./target for component: "
                .bright_red()
                .bold(),
            compose_name,
            e.to_string()
        );
        e
    })?;
    Ok(())
}
/// Reads contract file (lib.rs) as a text.
fn read_contracts_file_as_text(crate_metadata: &CrateMetadata) -> String {
    let target_dir = &crate_metadata.cargo_meta.target_directory;
    let mut composable_contract_source_path: PathBuf = target_dir.to_path_buf();
    composable_contract_source_path.pop();
    composable_contract_source_path.push("lib");
    composable_contract_source_path.set_extension("rs");

    fs::read_to_string(composable_contract_source_path)
        .expect("Something went wrong reading the composable contracts file")
}

/// Executes build of the smart-contract which produces a wasm binary that is ready for deploying.
///
/// It does so by invoking `cargo build` and then post processing the final binary.
///
/// # Note
///
/// Collects the contract crate's metadata using the supplied manifest (`Cargo.toml`) path. Use
/// [`execute_build_with_metadata`] if an instance is already available.
pub(crate) fn execute(
    manifest_path: &ManifestPath,
    verbosity: Option<Verbosity>,
    unstable_options: UnstableFlags,
) -> Result<PathBuf> {
    let crate_metadata = CrateMetadata::collect(manifest_path)?;

    let composable_schedule = crate_metadata.clone().t3rn_composable_schedule
        .expect("Failed to read composable metadata from JSON using serde. Make sure your Cargo.toml follows the composable metadata format");

    let mut compose_dest_path = Err(anyhow::anyhow!(
        "Empty composable t3rn contracts schedule. Didn't compile anything."
    ));
    // Scan for Wasm text format components.
    let contents = read_contracts_file_as_text(&crate_metadata);
    let mut contracts_scanner = WatContractsScanner::new_empty();
    contracts_scanner.scan_from_text(Box::leak(contents.into_boxed_str()));

    for compose in composable_schedule.composables.clone() {
        compose_dest_path = execute_with_metadata_composable(
            &crate_metadata,
            &mut contracts_scanner,
            compose,
            verbosity,
            unstable_options.clone(),
        );
    }
    match compose_dest_path {
        Ok(_) => Ok(crate_metadata.target_directory),
        Err(err) => Err(err),
    }
}
/// Executes build of the smart-contract which produces a wasm binary that is ready for deploying.
///
/// It does so by invoking `cargo build` and then post processing the final binary.
///
/// # Note
///
/// Uses the supplied `CrateMetadata`. If an instance is not available use [`execute_build`]
pub(crate) fn execute_with_metadata_composable(
    crate_metadata: &CrateMetadata,
    wat_contracts_scanner: &mut WatContractsScanner,
    compose: String,
    verbosity: Option<Verbosity>,
    unstable_options: UnstableFlags,
) -> Result<PathBuf> {
    match wat_contracts_scanner.find_by_name(compose.clone()) {
        Some(wat_contract) => {
            println!(
                "{} {} {}",
                "[1/3]".bold(),
                "Skipping cargo build; found WAT contract - component:"
                    .bright_green()
                    .bold(),
                compose.as_str().bright_green().bold()
            );
            println!(
                "{} {} {}",
                "[2/3]".bold(),
                "Compiling scanned WAT contract to WASM - component:"
                    .bright_green()
                    .bold(),
                compose.as_str().bright_green().bold()
            );
            compile_wat_to_wasm(compose.clone(), &crate_metadata, wat_contract)?;
        }
        None => {
            println!(
                "{} {} {}",
                "[1/3]".bold(),
                "Building cargo project - component:".bright_green().bold(),
                compose.as_str().bright_green().bold()
            );
            build_cargo_project_compose(
                &crate_metadata,
                compose.clone(),
                verbosity,
                unstable_options,
            )?;
            println!(
                " {} {} {}",
                "[2/3]".bold(),
                "Post processing wasm file - component:"
                    .bright_green()
                    .bold(),
                compose.as_str().bright_green().bold()
            );
            post_process_wasm_compose(&crate_metadata, compose.clone())?;
        }
    };

    println!(
        " {} {} {}",
        "[3/3]".bold(),
        "Optimizing wasm file - component:".bright_green().bold(),
        compose.as_str().bright_green().bold(),
    );
    optimize_wasm_compose(&crate_metadata, compose.clone())?;
    Ok(crate_metadata.dest_wasm.clone())
}

#[cfg(feature = "test-ci-only")]
#[cfg(test)]
mod tests {
    use crate::{cmd, util::tests::with_tmp_dir, workspace::ManifestPath, UnstableFlags};

    #[test]
    fn build_template() {
        with_tmp_dir(|path| {
            cmd::new::execute("new_project", Some(path)).expect("new project creation failed");
            let manifest_path =
                ManifestPath::new(&path.join("new_project").join("Cargo.toml")).unwrap();
            super::execute(&manifest_path, None, UnstableFlags::default()).expect("build failed");
            Ok(())
        })
    }
}
