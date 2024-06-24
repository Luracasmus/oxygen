use std::{fmt, thread::sleep, time::Duration};

pub fn panic_fancy<T>(msg: &str) -> T {
	eprintln!("Oxygen encountered an error!\n{msg}");

	println!("Closing automatically in 5 seconds...");
	sleep(Duration::from_secs(5));
	panic!()
}

pub trait FancyError<T, E> where E: fmt::Display {
	fn expect_fancy(self, msg: &str) -> T;
}

impl<T, E> FancyError<T, E> for Result<T, E> where E: fmt::Display {
	fn expect_fancy(self, msg: &str) -> T where E: fmt::Display {
		self.unwrap_or_else(|e| panic_fancy(&format!("{msg}\n{e}")))
	}
}

pub trait FancyNone<T> {
	fn expect_fancy(self, msg: &str) -> T;
}

impl<T> FancyNone<T> for Option<T> {
	fn expect_fancy(self, msg: &str) -> T {
		self.unwrap_or_else(|| panic_fancy(msg))
	}
}