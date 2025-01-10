use std::{env::{self, consts::EXE_SUFFIX}, ffi::OsStr, fs::File, io, path::{Path, PathBuf}, process::Command};

use serde::Deserialize;
use windows::Win32::System::Console;

#[derive(Deserialize)]
struct Manifest {
	name: Box<str>,
	path: Crate,
	features: Features
}

#[derive(Deserialize)]
enum Crate {
	CratesIO(Box<str>),
	Git {
		url: Box<str>,
		path: Option<Box<str>>
	},
	Local(Box<Path>)
}

#[derive(Deserialize)]
enum Features {
	Default,
	None,
	All,
	Add(Box<[Box<str>]>),
	Replace(Box<[Box<str>]>)
}

fn get_manifest() -> Option<Manifest> {
	let path = PathBuf::from(env::args().nth(1).or_else(|| {
		println!("No manifest path found in arguments"); None
	})?).into_boxed_path();

	if !path.exists() {
		println!("Supplied manifest path, '{path:?}', is invalid");
		return None;
	}

	if path.is_dir() {
		println!("Supplied manifest path, '{path:?}', is a directory, not a file");
		return None;
	}

	if path.extension() != Some(OsStr::new("o2")) {
		println!("Supplied manifest path, '{path:?}', has an unknown file extension. Expected `o2`\nProceeding anyway...");
	}

	let file = match File::open(&path) {
		Ok(file) => Some(file),
		Err(err) => {
			println!("Failed to open manifest at '{path:?}'\n{err}");
			None
		}
	}?;

	match ron::de::from_reader(file) {
		Ok(manifest) => Some(manifest),
		Err(err) => {
			println!("Failed to deserialize manifest at '{path:?}'! Is the file corrupted?\n{err}");
			None
		}
	}
}

fn update_rustup(problem: &mut bool) {
	if let Err(err) = Command::new("rustup").arg("update").status() {
		*problem = true;
		println!("Rustup Update failed!\n{err}");
	}
}

fn install(manifest: &Manifest, root: &str, problem: &mut bool) {
	update_rustup(problem);

	let mut install = Command::new("cargo");
		install.args([
			"install",
			"--root", root,
			"--target-dir", &(root.to_string() + "/cache"),
			"--config", "rustflags = [\"-C\", \"target-cpu=native\"]",

			"--config", "profile.release.lto = \"thin\"",
			"--config", "profile.release.opt-level = 3",
			"--config", "profile.release.debug = false",
			"--config", "profile.release.debug-assertions = false",
			"--config", "profile.release.overflow-checks = false",
			"--config", "profile.release.strip = \"symbols\"",
			"--config", "profile.release.codegen-units = 1",

			"--config", "profile.release.package.\"*\".opt-level = 3",
			"--config", "profile.release.package.\"*\".debug = false",
			"--config", "profile.release.package.\"*\".debug-assertions = false",
			"--config", "profile.release.package.\"*\".overflow-checks = false",
			"--config", "profile.release.package.\"*\".strip = \"symbols\"",
			"--config", "profile.release.package.\"*\".codegen-units = 1"
		]);

	match &manifest.path {
		Crate::CratesIO(name) => { install.arg(name.as_ref()); },
		Crate::Git{url, path} => {
			install.args(["--git", url]);

			if let Some(path) = path { install.arg(path.as_ref()); }
		},
		Crate::Local(path) => { install.args(["--path", path.to_str().unwrap()]); },
	};

	match &manifest.features {
		Features::Default => (),
		Features::None => { install.arg("--no-default-features"); },
		Features::All => { install.arg("--all-features"); },
		Features::Add(features) => { install.args(features.iter().map(AsRef::as_ref)); },
		Features::Replace(features) => { install.arg("--no-default-features").args(features.iter().map(AsRef::as_ref)); }
	}

	if let Err(err) = install.status() {
		*problem = true;
		println!("Cargo failed to install the application!\n{err}");
	}
}

fn run(path: &Path, always_show_console: bool, problem: &mut bool) {
	let mut app = Command::new(path);

	// Remove Terminal window when spawning
	#[cfg(target_os = "windows")]
	std::os::windows::process::CommandExt::creation_flags(&mut app, 0x0800_0000);

	let app = app.spawn();
	match app {
		Err(err) => {
			*problem = true;
			println!("Failed to spawn app!\n{err}");
		}
		Ok(mut app) => {
			#[cfg(target_os = "windows")]
			unsafe { Console::FreeConsole() }.unwrap();

			let app_result = app.wait();

			#[cfg(target_os = "windows")]
			if always_show_console {
				unsafe { Console::AllocConsole() }.unwrap();
			}

			if let Err(err) = app_result {
				#[cfg(target_os = "windows")]
				if !always_show_console {
					unsafe { Console::AllocConsole() }.unwrap();
				}

				*problem = true;
				println!("Failed to spawn app!\n{err}");
			}
		}
	}
}

fn main() {
	let mut problem = false;

	if let Some(manifest) = get_manifest() {
		let current_exe = env::current_exe().unwrap().into_boxed_path();
		let install_root = current_exe.parent().unwrap().to_str().unwrap();

		let app_path = PathBuf::from(&format!("{install_root}/bin/{}{EXE_SUFFIX}", manifest.name)).into_boxed_path();

		if app_path.exists() {
			run(&app_path, true, &mut problem);
			install(&manifest, install_root, &mut problem);
		} else {
			install(&manifest, install_root, &mut problem);
			run(&app_path, false, &mut problem);
		}
	} else {
		problem = true;
		println!("Failed to parse manifest. App update/startup cancelled");

		update_rustup(&mut problem);
	}

	if problem {
		println!("Press enter to exit");
		let mut temp = String::new();
		io::stdin().read_line(&mut temp).unwrap();
	}
}