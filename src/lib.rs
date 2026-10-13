//! Configuration Variables allow humans to interactively change the state of the program.
//!
//! Let's use an example to see how we can make it interactive.
//! The following snippet defines our program state with a user name and a method to greet the user:
//!
//! ```
//! pub struct User {
//! 	name: String,
//! }
//!
//! impl User {
//! 	pub fn greet(&self, writer: &mut dyn cvar::IWrite) {
//! 		let _ = writeln!(writer, "Hello, {}!", self.name);
//! 	}
//! }
//! ```
//!
//! Implement the [`IVisit`] trait to make this structure available for interactivity:
//!
//! ```
//! # struct User { name: String } impl User { pub fn greet(&self, writer: &mut dyn cvar::IWrite) { let _ = writeln!(writer, "Hello, {}!", self.name); } }
//! impl cvar::IVisit for User {
//! 	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
//! 		f(&mut cvar::Property("name", &mut self.name, &String::new()));
//! 		f(&mut cvar::Action("greet!", |_args, writer| self.greet(writer)));
//! 	}
//! }
//! ```
//!
//! That's it! Create an instance of the structure to interact with:
//!
//! ```
//! # struct User { name: String } impl User { pub fn greet(&self, writer: &mut dyn cvar::IWrite) { let _ = writeln!(writer, "Hello, {}!", self.name); } }
//! let mut user = User {
//! 	name: String::new(),
//! };
//! ```
//!
//! Given unique access, interact with the instance with a stringly typed API:
//!
//! ```
//! # struct User { name: String } impl User { pub fn greet(&self, writer: &mut dyn cvar::IWrite) { let _ = writeln!(writer, "Hello, {}!", self.name); } }
//! # impl cvar::IVisit for User { fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) { f(&mut cvar::Property("name", &mut self.name, &String::new())); f(&mut cvar::Action("greet!", |_args, writer| self.greet(writer))); } }
//! # let mut user = User { name: String::new() };
//! let mut writer = String::new();
//!
//! // Give the user a name
//! cvar::repl::set(&mut user, "name", "World", &mut writer);
//! assert_eq!(user.name, "World");
//!
//! // Greet the user, the message is printed to the writer
//! cvar::repl::invoke(&mut user, "greet!", "", &mut writer);
//! assert_eq!(writer, "Hello, World!\n");
//! ```
//!
//! This example is extremely basic, for more complex scenarios see the examples.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use core::{any, error, fmt, num, str::FromStr};

#[cfg(feature = "std")]
use std::io;

pub mod repl;

mod enums;
pub use enums::ParseEnumError;

mod value;
pub use self::value::*;

mod writer;
pub use self::writer::*;

mod action;
pub use self::action::*;

mod property;
pub use self::property::*;

mod list;
pub use self::list::*;

mod node;
pub use self::node::*;

#[inline]
fn write_error<T: ?Sized + error::Error>(writer: &mut dyn IWrite, v: &T) -> fmt::Result {
	writer.write_fmt(format_args!("{}", v))
}

#[cfg(feature = "type_name")]
#[inline]
fn write_mismatched_types<T: IValue>(writer: &mut dyn IWrite, val: &dyn IValue) -> fmt::Result {
	write!(writer, "mismatched types: expected `{}`, found `{}`", any::type_name::<T>(), val.type_name())
}

#[cfg(not(feature = "type_name"))]
#[inline]
fn write_mismatched_types<T: IValue>(writer: &mut dyn IWrite, _val: &dyn IValue) -> fmt::Result {
	writer.write_str("mismatched types")
}

#[cfg(test)]
mod tests;
