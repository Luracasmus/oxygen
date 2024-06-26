use std::{env, ffi::OsStr, fs::File, path::{Path, PathBuf}, process::Command, thread::sleep, time::Duration};

use serde::Deserialize;

const EXE_EXT: &str = if cfg!(target_os = "windows") { "exe" } else { "" };

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

fn main() {
	let mut problem = false;
	
	let update = match Command::new("rustup").arg("update").spawn() {
		Ok(update) => Some(update),
		Err(err) => {
			problem = true;
			println!("Failed to spawn 'rustup update'\n{err}\nProceeding anyway...");
			None
		}
	};

	if let Some(manifest) = get_manifest() {
		let current_exe = env::current_exe().unwrap().into_boxed_path();
		let install_root = current_exe.parent().unwrap().to_str().unwrap();

		let mut install = Command::new("cargo");
		install.args([
			"install",
			"--root", install_root,
			"--target-dir", &(install_root.to_string() + "/cache"),
			"--config", "rustflags = [\"-C\", \"target-cpu=native\", \"-C\", \"link-arg=-fuse-ld=lld\"]",

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

		match manifest.path {
			Crate::CratesIO(name) => { install.arg(name.as_ref()); },
			Crate::Git{url, path} => {
				install.args(["--git", &url]);

				if let Some(path) = path { install.arg(path.as_ref()); }
			},
			Crate::Local(path) => { install.args(["--path", path.to_str().unwrap()]); },
		};

		match manifest.features {
			Features::Default => (),
			Features::None => { install.arg("--no-default-features"); },
			Features::All => { install.arg("--all-features"); },
			Features::Add(features) => { install.args(features.iter().map(AsRef::as_ref)); },
			Features::Replace(features) => { install.arg("--no-default-features").args(features.iter().map(AsRef::as_ref)); }
		}

		if let Some(mut update) = update {
			if let Err(err) = update.wait() {
				problem = true;
				println!("Rustup Update failed!\n{err}\nProceeding anyway...");
			}
		}

		let install = install.spawn();

		let mut app = Command::new(format!("{install_root}/bin/{}.{EXE_EXT}", manifest.name));

		// Remove Terminal window when spawning app
		#[cfg(target_os = "windows")]
		std::os::windows::process::CommandExt::creation_flags(&mut app, 0x0800_0000);

		match install {
			Ok(mut install) => match install.wait() {
				Ok(_) => (),
				Err(err) => {
					problem = true;
					println!("Cargo Install failed!\n{err}\nAttempting to spawn app anyway...");
				}
			},
			Err(err) => {
				problem = true;
				println!("Failed to spawn 'cargo install'!\n{err}\nAttempting to spawn app anyway...");
			}
		}

		if let Err(err) = app.spawn() {
			problem = true;
			println!("Failed to spawn app!\n{err}");
		}
	} else {
		problem = true;
		println!("Aborting app installation/startup\nWaiting for Rustup Update to finish...");

		if let Some(mut update) = update {
			if let Err(err) = update.wait() {
				println!("Rustup Update failed!\n{err}");
			}
		}
	}

	if problem {
		println!("Exiting automatically in 10 seconds...");
		sleep(Duration::from_secs(10));
	}
}