/*!
Configuration Variables allow humans to interactively change the state of the program.

Let's use an example to see how we can make it interactive.
The following snippet defines our program state with a user name and a method to greet the user:

```
pub struct User {
	name: String,
}

impl User {
	pub fn greet(&self, writer: &mut dyn cvar::IWrite) {
		let _ = writeln!(writer, "Hello, {}!", self.name);
	}
}
```

Implement the [`IVisit`] trait to make this structure available for interactivity:

```
# struct User { name: String } impl User { pub fn greet(&self, writer: &mut dyn cvar::IWrite) { let _ = writeln!(writer, "Hello, {}!", self.name); } }
impl cvar::IVisit for User {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Property("name", &mut self.name, &String::new()));
		f(&mut cvar::Action("greet!", |_args, writer| self.greet(writer)));
	}
}
```

That's it! Create an instance of the structure to interact with:

```
# struct User { name: String } impl User { pub fn greet(&self, writer: &mut dyn cvar::IWrite) { let _ = writeln!(writer, "Hello, {}!", self.name); } }
let mut user = User {
	name: String::new(),
};
```

Given unique access, interact with the instance with a stringly typed API:

```
# struct User { name: String } impl User { pub fn greet(&self, writer: &mut dyn cvar::IWrite) { let _ = writeln!(writer, "Hello, {}!", self.name); } }
# impl cvar::IVisit for User { fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) { f(&mut cvar::Property("name", &mut self.name, &String::new())); f(&mut cvar::Action("greet!", |_args, writer| self.greet(writer))); } }
# let mut user = User { name: String::new() };
let mut writer = String::new();

// Give the user a name
cvar::console::set(&mut user, "name", "World", &mut writer);
assert_eq!(user.name, "World");

// Greet the user, the message is printed to the writer
cvar::console::invoke(&mut user, "greet!", "", &mut writer);
assert_eq!(writer, "Hello, World!\n");
```

This example is extremely basic, for more complex scenarios see the examples.
*/

use std::{any, error::Error as StdError, fmt, num, io, str::FromStr};

pub mod console;

#[cfg(test)]
mod tests;

//----------------------------------------------------------------

/// Node interface.
pub trait INode {
	/// Returns the node name.
	fn name(&self) -> &str;

	/// Downcasts to a more specific node interface.
	fn as_node(&mut self) -> Node<'_>;

	/// Upcasts back to an `INode` trait object.
	fn as_inode(&mut self) -> &mut dyn INode;
}

/// Enumerates derived interfaces for downcasting.
#[derive(Debug)]
pub enum Node<'a> {
	Prop(&'a mut dyn IProperty),
	List(&'a mut dyn IList),
	Action(&'a mut dyn IAction),
}

impl INode for Node<'_> {
	fn name(&self) -> &str {
		match self {
			Node::Prop(prop) => prop.name(),
			Node::List(list) => list.name(),
			Node::Action(act) => act.name(),
		}
	}

	fn as_node(&mut self) -> Node<'_> {
		match self {
			Node::Prop(prop) => Node::Prop(*prop),
			Node::List(list) => Node::List(*list),
			Node::Action(act) => Node::Action(*act),
		}
	}

	fn as_inode(&mut self) -> &mut dyn INode {
		self
	}
}

//----------------------------------------------------------------

/// Property values.
pub trait IValue: any::Any + fmt::Display {
	/// Returns the value as a `&dyn Any` trait object.
	fn as_any(&self) -> &dyn any::Any;

	/// Returns the name of the concrete type.
	#[cfg(feature = "type_name")]
	fn type_name(&self) -> &str {
		any::type_name::<Self>()
	}
}
impl fmt::Debug for dyn IValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, f)
	}
}

impl dyn IValue {
	/// Returns `true` if the inner type is the same as `T`.
	#[inline]
	pub fn is<T: any::Any>(&self) -> bool {
		any::TypeId::of::<T>() == self.type_id()
	}

	/// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
	#[inline]
	pub fn downcast_ref<T: any::Any>(&self) -> Option<&T> {
		if self.is::<T>() {
			Some(unsafe { &*(self as *const dyn IValue as *const T) })
		}
		else {
			None
		}
	}
}

impl<T: 'static + Sized> IValue for T
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: StdError + Send + Sync + 'static
{
	fn as_any(&self) -> &dyn any::Any {
		self
	}
}

//----------------------------------------------------------------

/// Format the value as hexadecimal.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct HexValue<T>(pub T);

impl<T> HexValue<T> {
	/// Transmutes to a `&HexValue<T>`.
	#[inline]
	pub fn from_ref(value: &T) -> &Self {
		unsafe { &*(value as *const T as *const Self) }
	}
	/// Transmutes to a `&mut HexValue<T>`.
	#[inline]
	pub fn from_mut(value: &mut T) -> &mut Self {
		unsafe { &mut *(value as *mut T as *mut Self) }
	}
}

impl<T> From<T> for HexValue<T> {
	#[inline]
	fn from(value: T) -> Self {
		Self(value)
	}
}
impl<T> AsRef<T> for HexValue<T> {
	#[inline]
	fn as_ref(&self) -> &T {
		&self.0
	}
}
impl<T> AsMut<T> for HexValue<T> {
	#[inline]
	fn as_mut(&mut self) -> &mut T {
		&mut self.0
	}
}

macro_rules! impl_HexValue {
	($ty:ty) => {
		impl fmt::Display for HexValue<$ty> {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				if self.0 == 0 {
					f.write_str("0")
				}
				else {
					write!(f, "{:#x}", self.0)
				}
			}
		}

		impl FromStr for HexValue<$ty> {
			type Err = num::ParseIntError;
			fn from_str(mut s: &str) -> Result<Self, num::ParseIntError> {
				let mut not = false;
				if let Some(rest) = s.strip_prefix("!") {
					s = rest;
					not = true;
				}

				let mut negate = false;
				if let Some(rest) = s.strip_prefix("-") {
					s = rest;
					negate = true;
				}

				let mut radix = 10;
				if let Some(rest) = s.strip_prefix("0x") {
					s = rest;
					radix = 16;
				}

				let mut value = <$ty>::from_str_radix(s, radix)?;

				if negate {
					value = value.wrapping_neg();
				}

				if not {
					value = !value;
				}

				Ok(HexValue(value))
			}
		}
	};
}

impl_HexValue!(u64);
impl_HexValue!(u32);
impl_HexValue!(u16);
impl_HexValue!(u8);

impl_HexValue!(i64);
impl_HexValue!(i32);
impl_HexValue!(i16);
impl_HexValue!(i8);

#[allow(non_snake_case)]
#[inline]
pub fn HexProp<'a, 'x, T>(name: &'a str, value: &'x mut T, default: &'a T) -> crate::Property<'a, 'x, HexValue<T>> {
	crate::Property(name, HexValue::from_mut(value), HexValue::from_ref(default))
}

//----------------------------------------------------------------

/// Property state.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
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
	/// Gets the value.
	fn get_value(&self) -> &dyn IValue;

	/// Sets the value.
	fn set_value(&mut self, val: &dyn IValue, writer: &mut dyn IWrite) -> bool;

	/// Sets the value parsed from string.
	fn set(&mut self, val: &str, writer: &mut dyn IWrite) -> bool;

	/// Resets the value to its default.
	///
	/// If this operation fails (for eg. read-only properties), it does so silently.
	fn reset(&mut self);

	/// Gets the default value.
	fn default_value(&self) -> &dyn IValue;

	/// Returns the state of the property.
	fn state(&self) -> PropState;

	/// Returns the flags associated with the property.
	///
	/// The meaning of this value is defined by the caller.
	fn flags(&self) -> u32 {
		0
	}

	/// Returns the name of the concrete type.
	#[cfg(feature = "type_name")]
	fn type_name(&self) -> &str {
		any::type_name::<Self>()
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
		let mut debug = f.debug_struct("IProperty");
		debug.field("name", &self.name());
		debug.field("value", &self.get_value());
		debug.field("default", &self.default_value());
		debug.field("state", &self.state());
		debug.field("flags", &self.flags());
		#[cfg(feature = "type_name")]
		debug.field("type", &self.type_name());
		debug.field("values", &self.values());
		debug.finish_non_exhaustive()
	}
}

//----------------------------------------------------------------

/// Property node.
pub struct Property<'a, 'x, T: 'static> {
	name: &'a str,
	variable: &'x mut T,
	default: &'a T,
}

#[allow(non_snake_case)]
#[inline]
pub fn Property<'a, 'x, T>(name: &'a str, variable: &'x mut T, default: &'a T) -> Property<'a, 'x, T> {
	Property { name, variable, default }
}

impl<'a, 'x, T> Property<'a, 'x, T> {
	#[inline]
	pub fn new(name: &'a str, variable: &'x mut T, default: &'a T) -> Property<'a, 'x, T> {
		Property { name, variable, default }
	}
}

impl<'a, 'x, T> INode for Property<'a, 'x, T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: StdError + Send + Sync + 'static
{
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}

	fn as_inode(&mut self) -> &mut dyn INode {
		self
	}
}

impl<'a, 'x, T> IProperty for Property<'a, 'x, T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: StdError + Send + Sync + 'static
{
	fn get_value(&self) -> &dyn IValue {
		&*self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
		if let Some(val) = val.downcast_ref::<T>() {
			self.variable.clone_from(val);
			true
		}
		else {
			let _ = write_mismatched_types::<T>(writer, val);
			false
		}
	}

	fn set(&mut self, val: &str, writer: &mut dyn IWrite) -> bool {
		match val.parse::<T>() {
			Ok(val) => {
				*self.variable = val;
				true
			},
			Err(err) => {
				let _ = write_error(writer, &err);
				false
			},
		}
	}

	fn reset(&mut self) {
		self.variable.clone_from(&self.default);
	}

	fn default_value(&self) -> &dyn IValue {
		&*self.default
	}

	fn state(&self) -> PropState {
		match *self.variable == *self.default {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}

//----------------------------------------------------------------

#[inline]
fn check_bounds_inclusive<T: PartialOrd>(val: &T, min: Option<&T>, max: Option<&T>) -> bool {
	if let Some(min) = min {
		if *val >= *min {
			return true;
		}
		return false;
	}
	if let Some(max) = max {
		if *val <= *max {
			return true;
		}
		return false;
	}
	return true;
}

/// Property node with its value clamped.
pub struct ClampedProp<'a, 'x, T: 'static> {
	name: &'a str,
	variable: &'x mut T,
	default: &'a T,
	min: Option<&'a T>,
	max: Option<&'a T>,
}

#[allow(non_snake_case)]
#[inline]
pub fn ClampedProp<'a, 'x, T>(name: &'a str, variable: &'x mut T, default: &'a T, min: Option<&'a T>, max: Option<&'a T>) -> ClampedProp<'a, 'x, T> {
	ClampedProp { name, variable, default, min, max }
}

impl<'a, 'x, T> ClampedProp<'a, 'x, T> {
	#[inline]
	pub fn new(name: &'a str, variable: &'x mut T, default: &'a T, min: Option<&'a T>, max: Option<&'a T>) -> ClampedProp<'a, 'x, T> {
		ClampedProp { name, variable, default, min, max }
	}
}

impl<'a, 'x, T> INode for ClampedProp<'a, 'x, T>
	where T: Clone + Default + PartialEq + PartialOrd + fmt::Display + FromStr,
	      T::Err: StdError + Send + Sync + 'static
{
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}

	fn as_inode(&mut self) -> &mut dyn INode {
		self
	}
}

impl<'a, 'x, T> IProperty for ClampedProp<'a, 'x, T>
	where T: Clone + Default + PartialEq + PartialOrd + fmt::Display + FromStr,
	      T::Err: StdError + Send + Sync + 'static
{
	fn get_value(&self) -> &dyn IValue {
		&*self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
		if let Some(val) = val.downcast_ref::<T>() {
			if check_bounds_inclusive(val, self.min, self.max) {
				self.variable.clone_from(val);
			}
			true
		}
		else {
			let _ = write_mismatched_types::<T>(writer, val);
			false
		}
	}

	fn set(&mut self, val: &str, writer: &mut dyn IWrite) -> bool {
		match val.parse::<T>() {
			Ok(val) => {
				if check_bounds_inclusive(&val, self.min, self.max) {
					*self.variable = val;
				}
				true
			},
			Err(err) => {
				let _ = write_error(writer, &err);
				false
			},
		}
	}

	fn reset(&mut self) {
		self.variable.clone_from(&self.default);
	}

	fn default_value(&self) -> &dyn IValue {
		&*self.default
	}

	fn state(&self) -> PropState {
		match *self.variable == *self.default {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}

//----------------------------------------------------------------

/// Read-only property node.
pub struct ReadOnlyProp<'a, T: 'static> {
	name: &'a str,
	variable: &'a T,
	default: &'a T,
}

#[allow(non_snake_case)]
#[inline]
pub fn ReadOnlyProp<'a, T>(name: &'a str, variable: &'a T, default: &'a T) -> ReadOnlyProp<'a, T> {
	ReadOnlyProp { name, variable, default }
}

impl<'a, T> ReadOnlyProp<'a, T> {
	#[inline]
	pub fn new(name: &'a str, variable: &'a T, default: &'a T) -> ReadOnlyProp<'a, T> {
		ReadOnlyProp { name, variable, default }
	}
}

impl<'a, T: PartialEq + IValue> INode for ReadOnlyProp<'a, T> {
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}

	fn as_inode(&mut self) -> &mut dyn INode {
		self
	}
}

impl<'a, T: PartialEq + IValue> IProperty for ReadOnlyProp<'a, T> {
	fn get_value(&self) -> &dyn IValue {
		&*self.variable
	}

	fn set_value(&mut self, _val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
		let _ = writer.write_str("cannot set read-only property");
		false
	}

	fn set(&mut self, _val: &str, writer: &mut dyn IWrite) -> bool {
		let _ = writer.write_str("cannot set read-only property");
		false
	}

	fn reset(&mut self) {}

	fn default_value(&self) -> &dyn IValue {
		&*self.default
	}

	fn state(&self) -> PropState {
		match *self.variable == *self.default {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}

//----------------------------------------------------------------

/// Property node which owns its variable.
pub struct OwnedProp<T: 'static> {
	pub name: String,
	pub variable: T,
	pub default: T,
	_private: (),
}

#[allow(non_snake_case)]
#[inline]
pub fn OwnedProp<T>(name: String, variable: T, default: T) -> OwnedProp<T> {
	OwnedProp { name, variable, default, _private: () }
}

impl<T> OwnedProp<T> {
	#[inline]
	pub fn new(name: String, variable: T, default: T) -> OwnedProp<T> {
		OwnedProp { name, variable, default, _private: () }
	}
}

impl<T> INode for OwnedProp<T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: StdError + Send + Sync + 'static
{
	fn name(&self) -> &str {
		&self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}

	fn as_inode(&mut self) -> &mut dyn INode {
		self
	}
}

impl<T> IProperty for OwnedProp<T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: StdError + Send + Sync + 'static
{
	fn get_value(&self) -> &dyn IValue {
		&self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
		if let Some(val) = val.downcast_ref::<T>() {
			self.variable.clone_from(val);
			true
		}
		else {
			let _ = write_mismatched_types::<T>(writer, val);
			false
		}
	}

	fn set(&mut self, val: &str, writer: &mut dyn IWrite) -> bool {
		match val.parse::<T>() {
			Ok(val) => {
				self.variable = val;
				true
			},
			Err(err) => {
				let _ = write_error(writer, &err);
				false
			},
		}
	}

	fn reset(&mut self) {
		self.variable.clone_from(&self.default);
	}

	fn default_value(&self) -> &dyn IValue {
		&self.default
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
///
/// impl cvar::IVisit for Foo {
/// 	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
/// 		// Pass type-erased properties, lists and actions to the closure
/// 		f(&mut cvar::Property("data", &mut self.data, &42));
/// 	}
/// }
/// ```
pub trait IVisit {
	/// Visits the child nodes.
	///
	/// Callers may depend on the particular order in which the nodes are passed to the closure.
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode));
}

impl fmt::Debug for dyn IVisit + '_ {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// Cannot visit the children as we do not have unique access to self...
		f.debug_struct("IVisit").finish_non_exhaustive()
	}
}

/// Node visitor from closure.
///
/// The visitor trait [`IVisit`] requires a struct type to be implemented.
/// This wrapper type allows a visitor to be created out of a closure instead.
///
/// ```
/// let mut value = 0;
///
/// let mut visitor = cvar::Visit(|f| {
/// 	f(&mut cvar::Property("value", &mut value, &0));
/// });
///
/// let mut writer = String::new();
/// let _ = cvar::console::set(&mut visitor, "value", "42", &mut writer);
/// assert_eq!(value, 42);
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Visit<F: FnMut(&mut dyn FnMut(&mut dyn INode))>(pub F);

impl<F: FnMut(&mut dyn FnMut(&mut dyn INode))> IVisit for Visit<F> {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		let Self(this) = self;
		this(f)
	}
}

//----------------------------------------------------------------

/// List of child nodes.
///
/// You probably want to implement the [`IVisit`] trait instead of this one.
pub trait IList: INode {
	/// Returns a visitor trait object to visit the children.
	fn as_ivisit(&mut self) -> &mut dyn IVisit;
}

impl fmt::Debug for dyn IList + '_ {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("IList")
			.field("name", &self.name())
			.finish_non_exhaustive()
	}
}

//----------------------------------------------------------------

/// List node.
#[derive(Debug)]
pub struct List<'a, 'x> {
	name: &'a str,
	visitor: &'x mut dyn IVisit,
}

#[allow(non_snake_case)]
#[inline]
pub fn List<'a, 'x>(name: &'a str, visitor: &'x mut dyn IVisit) -> List<'a, 'x> {
	List { name, visitor }
}

impl<'a, 'x> List<'a, 'x> {
	#[inline]
	pub fn new(name: &'a str, visitor: &'x mut dyn IVisit) -> List<'a, 'x> {
		List { name, visitor }
	}
}

impl<'a, 'x> INode for List<'a, 'x> {
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::List(self)
	}

	fn as_inode(&mut self) -> &mut dyn INode {
		self
	}
}

impl<'a, 'x> IList for List<'a, 'x> {
	fn as_ivisit(&mut self) -> &mut dyn IVisit {
		self.visitor
	}
}

//----------------------------------------------------------------

/// Console interface for actions to writer output to.
pub trait IWrite: any::Any + fmt::Write {}

impl dyn IWrite {
	/// Returns `true` if the inner type is the same as `T`.
	#[inline]
	pub fn is<T: any::Any>(&self) -> bool {
		any::TypeId::of::<T>() == self.type_id()
	}

	/// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
	#[inline]
	pub fn downcast_ref<T: any::Any>(&self) -> Option<&T> {
		if self.is::<T>() {
			Some(unsafe { &*(self as *const dyn IWrite as *const T) })
		}
		else {
			None
		}
	}

	/// Returns some mutable reference to the inner value if it is of type `T`, or `None` if it isn't.
	#[inline]
	pub fn downcast_mut<T: any::Any>(&mut self) -> Option<&mut T> {
		if self.is::<T>() {
			Some(unsafe { &mut *(self as *mut dyn IWrite as *mut T) })
		}
		else {
			None
		}
	}
}

#[inline]
fn write_error<T: ?Sized + StdError>(writer: &mut dyn IWrite, v: &T) -> fmt::Result {
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

impl IWrite for String {}

/// Null writer.
///
/// Helper which acts as `dev/null`, any writes disappear in the void.
pub struct NullWriter;

impl fmt::Write for NullWriter {
	fn write_str(&mut self, _s: &str) -> fmt::Result { Ok(()) }
	fn write_char(&mut self, _c: char) -> fmt::Result { Ok(()) }
	fn write_fmt(&mut self, _args: fmt::Arguments) -> fmt::Result { Ok(()) }
}

impl IWrite for NullWriter {}

/// Io writer.
///
/// Helper which adapts to any `std::io::Write` objects such as stdout.
pub struct IoWriter<W>(pub W);

impl<W: io::Write> fmt::Write for IoWriter<W> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		let Self(this) = self;
		io::Write::write_all(this, s.as_bytes()).map_err(|_| fmt::Error)
	}
	fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
		let Self(this) = self;
		io::Write::write_fmt(this, args).map_err(|_| fmt::Error)
	}
}

impl<W: io::Write + 'static> IWrite for IoWriter<W> {}

impl IoWriter<io::Stdout> {
	#[inline]
	pub fn stdout() -> IoWriter<io::Stdout> {
		IoWriter(io::stdout())
	}
}

impl IoWriter<io::Stderr> {
	#[inline]
	pub fn stderr() -> IoWriter<io::Stderr> {
		IoWriter(io::stderr())
	}
}

//----------------------------------------------------------------

/// Action node interface.
///
/// Provides an object safe interface for actions, type erasing its implementation.
pub trait IAction: INode {
	/// Invokes the closure associated with the Action.
	///
	/// Given argument string and a console interface to writer output to.
	fn invoke(&mut self, args: &str, writer: &mut dyn IWrite);
}

impl fmt::Debug for dyn IAction + '_ {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("IAction")
			.field("name", &self.name())
			.finish_non_exhaustive()
	}
}

//----------------------------------------------------------------

/// Action node.
#[derive(Debug)]
pub struct Action<'a, F: FnMut(&str, &mut dyn IWrite)> {
	name: &'a str,
	invoke: F,
}

#[allow(non_snake_case)]
#[inline]
pub fn Action<'a, F: FnMut(&str, &mut dyn IWrite)>(name: &'a str, invoke: F) -> Action<'a, F> {
	Action { name, invoke }
}

impl<'a, F: FnMut(&str, &mut dyn IWrite)> Action<'a, F> {
	#[inline]
	pub fn new(name: &'a str, invoke: F) -> Action<'a, F> {
		Action { name, invoke }
	}
}

impl<'a, F: FnMut(&str, &mut dyn IWrite)> INode for Action<'a, F> {
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Action(self)
	}

	fn as_inode(&mut self) -> &mut dyn INode {
		self
	}
}

impl<'a, F: FnMut(&str, &mut dyn IWrite)> IAction for Action<'a, F> {
	fn invoke(&mut self, args: &str, writer: &mut dyn IWrite) {
		(self.invoke)(args, writer)
	}
}
