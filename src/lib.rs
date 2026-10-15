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
//! 	pub fn greet(&self, context: &mut dyn cvar::IContext) -> cvar::ActionResult {
//! 		writeln!(context, "Hello, {}!", self.name);
//! 		Ok(())
//! 	}
//! }
//! ```
//!
//! Implement the [`IVisit`] trait to make this structure available for interactivity:
//!
//! ```
//! # struct User { name: String } impl User { pub fn greet(&self, context: &mut dyn cvar::IContext) -> cvar::ActionResult { writeln!(context, "Hello, {}!", self.name); Ok(()) } }
//! impl cvar::IVisit for User {
//! 	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
//! 		f(&mut cvar::Property("name", &mut self.name, &String::new()));
//! 		f(&mut cvar::Action("greet!", |_args, context| self.greet(context)));
//! 	}
//! }
//! ```
//!
//! That's it! Create an instance of the structure to interact with:
//!
//! ```
//! # struct User { name: String } impl User { pub fn greet(&self, context: &mut dyn cvar::IContext) -> cvar::ActionResult { writeln!(context, "Hello, {}!", self.name); Ok(()) } }
//! let mut user = User {
//! 	name: String::new(),
//! };
//! ```
//!
//! Given unique access, interact with the instance with a stringly typed API:
//!
//! ```
//! # struct User { name: String } impl User { pub fn greet(&self, context: &mut dyn cvar::IContext) -> cvar::ActionResult { writeln!(context, "Hello, {}!", self.name); Ok(()) } }
//! # impl cvar::IVisit for User { fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) { f(&mut cvar::Property("name", &mut self.name, &String::new())); f(&mut cvar::Action("greet!", |_args, context| self.greet(context))); } }
//! # let mut user = User { name: String::new() };
//! let mut context = String::new();
//!
//! // Give the user a name
//! cvar::repl::set(&mut user, "name", "World", &mut context).unwrap();
//! assert_eq!(user.name, "World");
//!
//! // Greet the user, the message is written to the context
//! cvar::repl::invoke(&mut user, "greet!", "", &mut context).unwrap();
//! assert_eq!(context, "Hello, World!\n");
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

mod context;
pub use self::context::*;

mod action;
pub use self::action::*;

mod property;
pub use self::property::*;

mod list;
pub use self::list::*;

mod node;
pub use self::node::*;

#[cfg(test)]
mod tests;
