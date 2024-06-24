#![warn(
	clippy::cargo,
	clippy::pedantic,
	clippy::nursery,

	clippy::filetype_is_file,
	clippy::float_cmp_const,
	clippy::fn_to_numeric_cast_any,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::mem_forget,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::string_to_string,
	clippy::suspicious_xor_used_as_pow,
	clippy::rc_mutex,
	clippy::ref_patterns,
	clippy::rest_pat_in_fully_bound_structs
)]

#![allow(
	clippy::cargo_common_metadata,
	clippy::cast_lossless,
	clippy::cast_possible_truncation,
	clippy::cast_possible_wrap,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::cognitive_complexity,
	clippy::integer_division,
	clippy::module_name_repetitions,
	clippy::multiple_crate_versions,
	clippy::needless_pass_by_value,
	clippy::too_many_lines,
	clippy::wildcard_imports
)]

mod panic;

use std::{env, ffi::OsStr, fs::File, path::{Path, PathBuf}, process::Command};

use panic::*;
use serde::Deserialize;

const EXE_EXT: &str = if cfg!(target_os = "windows") { "exe" } else { "" };

#[derive(Deserialize)]
struct Manifest {
	name: String,
	path: Crate,
	features: Features
}

#[derive(Deserialize)]
enum Crate {
	CratesIO(String),
	Git {
		url: String,
		path: Option<String>
	},
	Local(PathBuf)
}

#[derive(Deserialize)]
enum Features {
	Default,
	None,
	All,
	Add(Vec<String>),
	Replace(Vec<String>)
}

fn remove_window(command: &mut Command) -> &mut Command {
	#[cfg(target_os = "windows")]
	std::os::windows::process::CommandExt::creation_flags(command, 0x0800_0000)
}

fn main() {
	println!("--| Oxygen {} |--\nUpdating Rust (Rustup):", env!("CARGO_PKG_VERSION"));

	Command::new(Path::new("rustup"))
		.arg("update")
		.spawn().expect_fancy("Failed to spawn 'rustup update'. Is Rustup installed?") // attempt 'winget install rustup' or 'curl https://sh.rustup.rs -sSf | sh'
		.wait().expect_fancy("'rustup update' failed");

	println!("Deserializing Manifest...");

	let path = PathBuf::from(env::args().nth(1).expect_fancy("No manifest path found"));
	if !path.is_file() { panic_fancy("Supplied manifest path does not lead to a file") };
	if path.extension() != Some(OsStr::new("o2")) { panic_fancy("Foreign file extension") }

	let manifest: Manifest = ron::de::from_reader(
		&File::open(&path).expect_fancy(&format!("Failed to open manifest at '{path:?}'"))
	).expect_fancy(&format!("Failed to read manifest at '{path:?}'. Is the file corrupted?"));

	println!("Installing Application (Cargo):");

	let current_exe = env::current_exe().unwrap();
	let install_root = current_exe.parent().unwrap().to_str().unwrap();

	let mut install = Command::new(Path::new("cargo"));
	install.args([
		"install",
		"--root", install_root,
		"--target-dir", &format!("{install_root}/cache"),
		"--config", "rustflags = [\"-C\", \"target-cpu=native\", \"-C\", \"link-arg=-fuse-ld=lld\"]",
		"--config", "profile.release.lto = \"thin\"",
		"--config", "profile.release.opt-level = 3",
		"--config", "profile.release.debug = false",
		"--config", "profile.release.debug-assertions = false",
		"--config", "profile.release.overflow-checks = false",
		"--config", "profile.release.strip = \"symbols\"",
		"--config", "profile.release.codegen-units = 1"
	]);

	match manifest.path {
		Crate::CratesIO(name) => { install.arg(name); },
		Crate::Git{url, path} => {
			install.args(["--git", url.as_str()]);

			if let Some(path) = path { install.arg(path); }
		},
		Crate::Local(path) => { install.args(["--path", path.to_str().unwrap()]); },
	};

	match manifest.features {
		Features::Default => (),
		Features::None => { install.arg("--no-default-features"); },
		Features::All => { install.arg("--all-features"); },
		Features::Add(features) => { install.args(features); },
		Features::Replace(features) => { install.arg("--no-default-features").args(features); }
	}

	install
		.spawn().expect_fancy("Failed to spawn 'cargo install'. Is Cargo installed?")
		.wait().expect_fancy("'cargo install' failed");

	println!("Spawning Application...");

	let app = format!("{install_root}/bin/{}.{EXE_EXT}", manifest.name);
	remove_window(&mut Command::new(&app)).spawn().expect_fancy(&format!("Failed to spawn {app}"));

	//println!("Exiting automatically in 3 seconds...");
	//sleep(Duration::from_secs(3));
}