# Oxygen
"Package manager" compiling crates with native CPU features and maximal optimizations

To install a crate, run Oxygen with the path to an Oxygen Manifest (in RON format), such as this:

`circles.o2`:
```ron
(
	name: "circles",
	path: Git(
		url: "https://github.com/Luracasmus/circles.git",
		path: None
	),
	features: Default
)
```
`> oxygen "path/to/circles.o2"`

The crate will be installed in a directory next to Oxygen itself, and can be started by simply opening the manifest again (you can even associate Oxygen with the `o2` file extension to make the manifest work almost like a shortcut!). Every time the manifest is ran, the crate, along with Rustup and the Rust toolchain are automatically updated. If the crate is already installed, updates run after it exits to minimize launch time. All compiled dependencies are cached and shared between crates

Oxygen requires Rustup and Cargo to be installed
