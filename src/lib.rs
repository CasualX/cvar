/*!
Configuration Variables allow humans to interactively change the state of the program.

Let's use an example to see how we can make it interactive.
The following snippet defines our program state with a user name and a method to greet the user:

```
pub struct User {
	name: String,
}
impl User {
	pub fn greet(&self, console: &mut cvar::IConsole) {
		let _ = writeln!(console, "Hello, {}!", self.name);
	}
}
```

Implement [the `IVisit` trait](trait.IVisit.html) to make this structure available for interactivity:

```
# struct User { name: String } impl User { pub fn greet(&self, console: &mut cvar::IConsole) { let _ = writeln!(console, "Hello, {}!", self.name); } }
impl cvar::IVisit for User {
	fn visit_mut(&mut self, f: &mut FnMut(&mut cvar::INode)) {
		f(&mut cvar::Property("name", "description", &mut self.name, String::new()));
		f(&mut cvar::Action("greet!", "description", |_args, console| self.greet(console)));
	}
}
```

That's it! Create an instance of the structure to interact with:

```
# struct User { name: String } impl User { pub fn greet(&self, console: &mut cvar::IConsole) { let _ = writeln!(console, "Hello, {}!", self.name); } }
let mut user = User {
	name: String::new(),
};
```

Given unique access, interact with the instance with a stringly typed API:

```
# struct User { name: String } impl User { pub fn greet(&self, console: &mut cvar::IConsole) { let _ = writeln!(console, "Hello, {}!", self.name); } }
# impl cvar::IVisit for User { fn visit_mut(&mut self, f: &mut FnMut(&mut cvar::INode)) { f(&mut cvar::Property("name", "description", &mut self.name, String::new())); f(&mut cvar::Action("greet!", "description", |_args, console| self.greet(console))); } }
# let mut user = User { name: String::new() };
// Give the user a name
cvar::console::set(&mut user, "name", "World").unwrap();
assert_eq!(user.name, "World");

// Greet the user, the message is printed to the console string
let mut console = String::new();
cvar::console::invoke(&mut user, "greet!", &[""], &mut console);
assert_eq!(console, "Hello, World!\n");
```

This example is extremely basic, for more complex scenarios see the examples.
!*/

use std::{error::Error as StdError, fmt, io, str::FromStr};

pub mod console;

#[cfg(test)]
mod tests;

/// Result with boxed error.
type BoxResult<T> = Result<T, Box<dyn StdError + Send + Sync + 'static>>;

//----------------------------------------------------------------

/// Node interface.
///
/// Defines the basic requirements of a node such as having a name and a description.
pub trait INode {
	/// Returns the node name.
	fn name(&self) -> &str;
	/// Returns the node description.
	fn description(&self) -> &str;
	/// Downcasts to a more specific node interface.
	fn as_node_mut(&mut self) -> NodeMut<'_>;
	/// Upcasts back to an `INode` trait object.
	fn as_inode_mut(&mut self) -> &mut dyn INode;
}

/// Enumerates derived interfaces for downcasting.
#[derive(Debug)]
pub enum NodeMut<'a> {
	Prop(&'a mut dyn IProperty),
	List(&'a mut dyn IList),
	Action(&'a mut dyn IAction),
}
impl INode for NodeMut<'_> {
	fn name(&self) -> &str {
		match self {
			NodeMut::Prop(prop) => prop.name(),
			NodeMut::List(list) => list.name(),
			NodeMut::Action(act) => act.name(),
		}
	}
	fn description(&self) -> &str {
		match self {
			NodeMut::Prop(prop) => prop.description(),
			NodeMut::List(list) => list.description(),
			NodeMut::Action(act) => act.description(),
		}
	}
	fn as_node_mut(&mut self) -> NodeMut<'_> {
		match self {
			NodeMut::Prop(prop) => NodeMut::Prop(*prop),
			NodeMut::List(list) => NodeMut::List(*list),
			NodeMut::Action(act) => NodeMut::Action(*act),
		}
	}
	fn as_inode_mut(&mut self) -> &mut dyn INode {
		self
	}
}

//----------------------------------------------------------------

/// Property state.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PropState {
	/// The property has its default value set.
	Default,
	/// The property has a non-default value.
	UserSet,
	/// The value is not valid in the current context.
	Invalid,
}

/// Property node interface.
///
/// Provides an object safe interface for properties, type erasing its implementation.
pub trait IProperty: INode {
	/// Gets the value as a string.
	fn get(&self) -> String;
	/// Sets the value.
	fn set(&mut self, val: &str) -> BoxResult<()>;
	/// Resets the value to its default.
	///
	/// If this operation fails (for eg. read-only properties), it does so silently.
	fn reset(&mut self);
	/// Gets the default value as a string.
	fn default(&self) -> String;
	/// Returns the state of the property.
	fn state(&self) -> PropState;
	/// Returns the flags associated with the property.
	///
	/// The meaning of this value is defined by the caller.
	fn flags(&self) -> u32 {
		0
	}
	/// Returns a list of valid value strings for this property.
	///
	/// None if the question is not relevant, eg. string or number nodes.
	fn values(&self) -> Option<&[&str]> {
		None
	}
}
impl fmt::Debug for dyn IProperty + '_ {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("IProperty")
			.field("name", &self.name())
			.field("desc", &self.description())
			.field("value", &self.get())
			.field("default", &self.default())
			.field("state", &self.state())
			.field("flags", &self.flags())
			.field("values", &self.values())
			.finish()
	}
}

//----------------------------------------------------------------

/// Property node.
pub struct Property<'a, T> {
	name: &'a str,
	desc: &'a str,
	variable: &'a mut T,
	default: T,
}
#[allow(non_snake_case)]
pub fn Property<'a, T>(name: &'a str, desc: &'a str, variable: &'a mut T, default: T) -> Property<'a, T> {
	Property { name, desc, variable, default }
}
impl<'a, T> Property<'a, T> {
	pub fn new(name: &'a str, desc: &'a str, variable: &'a mut T, default: T) -> Property<'a, T> {
		Property { name, desc, variable, default }
	}
}
impl<'a, T> INode for Property<'a, T>
	where T: FromStr + ToString + Clone + PartialEq,
	      T::Err: StdError + Send + Sync + 'static
{
	fn name(&self) -> &str {
		self.name
	}
	fn description(&self) -> &str {
		self.desc
	}
	fn as_node_mut(&mut self) -> NodeMut<'_> {
		NodeMut::Prop(self)
	}
	fn as_inode_mut(&mut self) -> &mut dyn INode {
		self
	}
}
impl<'a, T> IProperty for Property<'a, T>
	where T: FromStr + ToString + Clone + PartialEq,
	      T::Err: StdError + Send + Sync + 'static
{
	fn get(&self) -> String {
		self.variable.to_string()
	}
	fn set(&mut self, val: &str) -> BoxResult<()> {
		*self.variable = T::from_str(val)?;
		Ok(())
	}
	fn reset(&mut self) {
		self.variable.clone_from(&self.default);
	}
	fn default(&self) -> String {
		self.default.to_string()
	}
	fn state(&self) -> PropState {
		match *self.variable == self.default {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}

//----------------------------------------------------------------

/// Property node with its value clamped.
pub struct ClampedProp<'a, T> {
	name: &'a str,
	desc: &'a str,
	variable: &'a mut T,
	default: T,
	min: T,
	max: T,
}
#[allow(non_snake_case)]
pub fn ClampedProp<'a, T>(name: &'a str, desc: &'a str, variable: &'a mut T, default: T, min: T, max: T) -> ClampedProp<'a, T> {
	ClampedProp { name, desc, variable, default, min, max }
}
impl<'a, T> ClampedProp<'a, T> {
	pub fn new(name: &'a str, desc: &'a str, variable: &'a mut T, default: T, min: T, max: T) -> ClampedProp<'a, T> {
		ClampedProp { name, desc, variable, default, min, max }
	}
}
impl<'a, T> INode for ClampedProp<'a, T>
	where T: FromStr + ToString + Clone + PartialEq + PartialOrd,
	      T::Err: StdError + Send + Sync + 'static
{
	fn name(&self) -> &str {
		self.name
	}
	fn description(&self) -> &str {
		self.desc
	}
	fn as_node_mut(&mut self) -> NodeMut<'_> {
		NodeMut::Prop(self)
	}
	fn as_inode_mut(&mut self) -> &mut dyn INode {
		self
	}
}
impl<'a, T> IProperty for ClampedProp<'a, T>
	where T: FromStr + ToString + Clone + PartialEq + PartialOrd,
	      T::Err: StdError + Send + Sync + 'static
{
	fn get(&self) -> String {
		self.variable.to_string()
	}
	fn set(&mut self, val: &str) -> BoxResult<()> {
		*self.variable = T::from_str(val)?;
		if *self.variable < self.min {
			self.variable.clone_from(&self.min);
		}
		if *self.variable > self.max {
			self.variable.clone_from(&self.max);
		}
		Ok(())
	}
	fn reset(&mut self) {
		self.variable.clone_from(&self.default);
	}
	fn default(&self) -> String {
		self.default.to_string()
	}
	fn state(&self) -> PropState {
		match *self.variable == self.default {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}

//----------------------------------------------------------------

/// Read-only property node.
pub struct ReadOnlyProp<'a, T> {
	name: &'a str,
	desc: &'a str,
	variable: &'a T,
	default: T,
}
#[allow(non_snake_case)]
pub fn ReadOnlyProp<'a, T>(name: &'a str, desc: &'a str, variable: &'a T, default: T) -> ReadOnlyProp<'a, T> {
	ReadOnlyProp { name, desc, variable, default }
}
impl<'a, T> ReadOnlyProp<'a, T> {
	pub fn new(name: &'a str, desc: &'a str, variable: &'a T, default: T) -> ReadOnlyProp<'a, T> {
		ReadOnlyProp { name, desc, variable, default }
	}
}
impl<'a, T: ToString + PartialEq> INode for ReadOnlyProp<'a, T> {
	fn name(&self) -> &str {
		self.name
	}
	fn description(&self) -> &str {
		self.desc
	}
	fn as_node_mut(&mut self) -> NodeMut<'_> {
		NodeMut::Prop(self)
	}
	fn as_inode_mut(&mut self) -> &mut dyn INode {
		self
	}
}
impl<'a, T: ToString + PartialEq> IProperty for ReadOnlyProp<'a, T> {
	fn get(&self) -> String {
		self.variable.to_string()
	}
	fn set(&mut self, _val: &str) -> BoxResult<()> {
		Err("cannot set read-only property".into())
	}
	fn reset(&mut self) {}
	fn default(&self) -> String {
		self.default.to_string()
	}
	fn state(&self) -> PropState {
		match *self.variable == self.default {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}

//----------------------------------------------------------------

/// Property node which owns its variable.
pub struct OwnedProp<T> {
	pub name: String,
	pub variable: T,
	pub default: T,
	_private: (),
}
#[allow(non_snake_case)]
pub fn OwnedProp<T>(name: String, variable: T, default: T) -> OwnedProp<T> {
	OwnedProp { name, variable, default, _private: () }
}
impl<T> OwnedProp<T> {
	pub fn new(name: String, variable: T, default: T) -> OwnedProp<T> {
		OwnedProp { name, variable, default, _private: () }
	}
}
impl<T> INode for OwnedProp<T>
	where T: FromStr + ToString + Clone + PartialEq,
	      T::Err: StdError + Send + Sync + 'static
{
	fn name(&self) -> &str { &self.name }
	fn description(&self) -> &str { "" }
	fn as_node_mut(&mut self) -> NodeMut<'_> { NodeMut::Prop(self) }
	fn as_inode_mut(&mut self) -> &mut dyn INode { self }
}
impl<T> IProperty for OwnedProp<T>
	where T: FromStr + ToString + Clone + PartialEq,
	      T::Err: StdError + Send + Sync + 'static
{
	fn get(&self) -> String {
		self.variable.to_string()
	}
	fn set(&mut self, val: &str) -> BoxResult<()> {
		self.variable = T::from_str(val)?;
		Ok(())
	}
	fn reset(&mut self) {
		self.variable.clone_from(&self.default);
	}
	fn default(&self) -> String {
		self.default.to_string()
	}
	fn state(&self) -> PropState {
		match self.variable == self.default {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}

//----------------------------------------------------------------

/// Node visitor.
///
/// The visitor pattern is used to discover child nodes in custom types.
///
/// This trait is most commonly required to be implemented by users of this crate.
///
/// ```
/// struct Foo {
/// 	data: i32,
/// }
/// impl cvar::IVisit for Foo {
/// 	fn visit_mut(&mut self, f: &mut FnMut(&mut cvar::INode)) {
/// 		// Pass type-erased properties, lists and actions to the closure
/// 		f(&mut cvar::Property("data", "description", &mut self.data, 42));
/// 	}
/// }
/// ```
pub trait IVisit {
	/// Visits the child nodes.
	///
	/// Callers may depend on the particular order in which the nodes are passed to the closure.
	fn visit_mut(&mut self, f: &mut dyn FnMut(&mut dyn INode));
}
impl fmt::Debug for dyn IVisit + '_ {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// Cannot visit the children as we do not have unique access to self...
		f.write_str("IVisit { .. }")
	}
}

/// Node visitor from closure.
///
/// The visitor trait `IVisit` requires a struct type to be implemented.
/// This wrapper type allows a visitor to be created out of a closure instead.
///
/// ```
/// let mut value = 0;
///
/// let mut visitor = cvar::VisitMut(|f| {
/// 	f(&mut cvar::Property("value", "description", &mut value, 0));
/// });
///
/// let _ = cvar::console::set(&mut visitor, "value", "42");
/// assert_eq!(value, 42);
/// ```
#[derive(Copy, Clone, Debug)]
pub struct VisitMut<F: FnMut(&mut dyn FnMut(&mut dyn INode))>(pub F);
impl<F: FnMut(&mut dyn FnMut(&mut dyn INode))> IVisit for VisitMut<F> {
	fn visit_mut(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		(self.0)(f)
	}
}

//----------------------------------------------------------------

/// List of child nodes.
///
/// An `IList` implements the node interface with its associated name and description metadata.
///
/// You probably want to implement [the `IVisit` trait](trait.IVisit.html) instead of this one.
pub trait IList: INode {
	/// Returns a visitor trait object to visit the children.
	fn as_visit_mut(&mut self) -> &mut dyn IVisit;
}
impl fmt::Debug for dyn IList + '_ {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("IList")
			.field("name", &self.name())
			.field("desc", &self.description())
			.finish()
	}
}

//----------------------------------------------------------------

/// List node.
#[derive(Debug)]
pub struct List<'a, V> {
	name: &'a str,
	desc: &'a str,
	visitor: &'a mut V,
}
#[allow(non_snake_case)]
pub fn List<'a, V>(name: &'a str, desc: &'a str, visitor: &'a mut V) -> List<'a, V> {
	List { name, desc, visitor }
}
impl<'a, V> List<'a, V> {
	pub fn new(name: &'a str, desc: &'a str, visitor: &'a mut V) -> List<'a, V> {
		List { name, desc, visitor }
	}
}
impl<'a, V: IVisit> INode for List<'a, V> {
	fn name(&self) -> &str {
		self.name
	}
	fn description(&self) -> &str {
		self.desc
	}
	fn as_node_mut(&mut self) -> NodeMut<'_> {
		NodeMut::List(self)
	}
	fn as_inode_mut(&mut self) -> &mut dyn INode {
		self
	}
}
impl<'a, V: IVisit> IList for List<'a, V> {
	fn as_visit_mut(&mut self) -> &mut dyn IVisit {
		self.visitor
	}
}

//----------------------------------------------------------------

/// Console interface for actions to write output to.
pub trait IConsole: fmt::Write {
	/// Notifies the console an error has occurred.
	fn write_error(&mut self, err: &(dyn StdError + 'static));
}

impl IConsole for String {
	fn write_error(&mut self, err: &(dyn StdError + 'static)) {
		let _ = writeln!(self as &mut dyn fmt::Write, "error: {}", err);
	}
}

/// Null console for actions.
///
/// Helper which acts as `dev/null`, any writes disappear in the void.
pub struct NullConsole;
impl fmt::Write for NullConsole {
	fn write_str(&mut self, _s: &str) -> fmt::Result { Ok(()) }
	fn write_char(&mut self, _c: char) -> fmt::Result { Ok(()) }
	fn write_fmt(&mut self, _args: fmt::Arguments) -> fmt::Result { Ok(()) }
}
impl IConsole for NullConsole {
	fn write_error(&mut self, _err: &(dyn StdError + 'static)) {}
}

/// Io console for actions.
///
/// Helper which adapts a console to write to any `std::io::Write` objects such as stdout.
pub struct IoConsole<W>(pub W);
impl<W: io::Write> fmt::Write for IoConsole<W> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		io::Write::write_all(&mut self.0, s.as_bytes()).map_err(|_| fmt::Error)
	}
	fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
		io::Write::write_fmt(&mut self.0, args).map_err(|_| fmt::Error)
	}
}
impl<W: io::Write> IConsole for IoConsole<W> {
	fn write_error(&mut self, err: &(dyn StdError + 'static)) {
		let _ = writeln!(self.0, "error: {}", err);
	}
}
impl IoConsole<io::Stdout> {
	pub fn stdout() -> IoConsole<io::Stdout> {
		IoConsole(io::stdout())
	}
}
impl IoConsole<io::Stderr> {
	pub fn stderr() -> IoConsole<io::Stderr> {
		IoConsole(io::stderr())
	}
}

//----------------------------------------------------------------

/// Action node interface.
///
/// Provides an object safe interface for actions, type erasing its implementation.
pub trait IAction: INode {
	/// Invokes the closure associated with the Action.
	///
	/// Given pre-tokenized arguments and a console interface to write output to.
	fn invoke(&mut self, args: &[&str], console: &mut dyn IConsole);
}
impl fmt::Debug for dyn IAction + '_ {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("IAction")
			.field("name", &self.name())
			.field("desc", &self.description())
			.finish()
	}
}

//----------------------------------------------------------------

/// Action node.
#[derive(Debug)]
pub struct Action<'a, F: FnMut(&[&str], &mut dyn IConsole)> {
	name: &'a str,
	desc: &'a str,
	invoke: F,
}
#[allow(non_snake_case)]
pub fn Action<'a, F: FnMut(&[&str], &mut dyn IConsole)>(name: &'a str, desc: &'a str, invoke: F) -> Action<'a, F> {
	Action { name, desc, invoke }
}
impl<'a, F: FnMut(&[&str], &mut dyn IConsole)> Action<'a, F> {
	pub fn new(name: &'a str, desc: &'a str, invoke: F) -> Action<'a, F> {
		Action { name, desc, invoke }
	}
}
impl<'a, F: FnMut(&[&str], &mut dyn IConsole)> INode for Action<'a, F> {
	fn name(&self) -> &str {
		self.name
	}
	fn description(&self) -> &str {
		self.desc
	}
	fn as_node_mut(&mut self) -> NodeMut<'_> {
		NodeMut::Action(self)
	}
	fn as_inode_mut(&mut self) -> &mut dyn INode {
		self
	}
}
impl<'a, F: FnMut(&[&str], &mut dyn IConsole)> IAction for Action<'a, F> {
	fn invoke(&mut self, args: &[&str], console: &mut dyn IConsole) {
		(self.invoke)(args, console)
	}
}
