/*!
Let us use an example to see how cvars might be implemented for it.

```
extern crate cvar;

use ::std::cell::{Cell, RefCell};

struct Foo {
	int: Cell<i32>,
	name: RefCell<String>,
}
impl Foo {
	fn greet(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		Ok(try!(writeln!(ctx.write, "Hello, {}!", *self.name.borrow())))
	}
}
```

Important is that this library is designed with passing non-mutable references around, thus configurable variables need interior mutability.

That is the basic setup, we would like to create these properties:

* `foo.int`: Property representing an `i32` variable.

* `foo.name`: The name used in the greeting.

* `foo.greet!`: An action that will print a greeting for `foo.name`. See the [`OnInvoke`](trait.OnInvoke.html) trait for more information about its parameters.

```
# use ::std::cell::{Cell, RefCell}; struct Foo { int: Cell<i32>, name: RefCell<String>, } impl Foo { fn greet(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> { Ok(try!(writeln!(ctx.write, "Hello, {}!", *self.name.borrow()))) } }
impl cvar::IVisit for Foo {
	fn visit(&self, f: &mut FnMut(cvar::Node)) {
		use cvar::{Property, Action};
		f(From::from(&Property::new("int", "int description", &self.int, 42)));
		f(From::from(&Property::new("name", "name description", &self.name, "Casper")));
		f(From::from(&Action::new("greet!", "action description", |ctx| self.greet(ctx))));
	}
}
```

Accessing children is done via the [`IVisit`](trait.IVisit.html) trait implementing the Visitor Pattern. Its implementation will invoke the callback with every child as a [`Node`](enum.Node.html).

```
# use ::std::cell::{Cell, RefCell}; struct Foo { int: Cell<i32>, name: RefCell<String>, } impl Foo { fn greet(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> { Ok(try!(writeln!(ctx.write, "Hello, {}!", *self.name.borrow()))) } }
# impl cvar::IVisit for Foo { fn visit(&self, f: &mut FnMut(cvar::Node)) { use cvar::{Property, Action}; f(From::from(&Property::new("int", "int description", &self.int, 42))); f(From::from(&Property::new("name", "name description", &self.name, "Casper"))); f(From::from(&Action::new("greet!", "action description", |ctx| self.greet(ctx)))); } }
struct Root {
	foo: Foo,
}
impl cvar::IVisit for Root {
	fn visit(&self, f: &mut FnMut(cvar::Node)) {
		use cvar::List;
		f(From::from(&List::new("foo", "foo description", &self.foo)));
	}
}
```

To access these cvars there is one thing missing: a root object from which they are reachable. Here modeled by having the root own a `Foo` instance.

An important note is that the root is not a list node, it does not have any metadata it just exists as a point where the rest of the cvars are accessible from.

```
# use ::std::cell::{Cell, RefCell}; struct Foo { int: Cell<i32>, name: RefCell<String>, } impl Foo { fn greet(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> { Ok(try!(writeln!(ctx.write, "Hello, {}!", *self.name.borrow()))) } }
# impl cvar::IVisit for Foo { fn visit(&self, f: &mut FnMut(cvar::Node)) { use cvar::{Property, Action}; f(From::from(&Property::new("int", "int description", &self.int, 42))); f(From::from(&Property::new("name", "name description", &self.name, "Casper"))); f(From::from(&Action::new("greet!", "action description", |ctx| self.greet(ctx)))); } }
# struct Root { foo: Foo, } impl cvar::IVisit for Root { fn visit(&self, f: &mut FnMut(cvar::Node)) { use cvar::List; f(From::from(&List::new("foo", "foo description", &self.foo))); } }
let root = Root {
	foo: Foo {
		int: Cell::new(13),
		name: RefCell::new(String::new()),
	},
};
```

That's it! Now we are almost ready, let us create an instance of the root.

```
# use ::std::cell::{Cell, RefCell}; struct Foo { int: Cell<i32>, name: RefCell<String>, } impl Foo { fn greet(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> { Ok(try!(writeln!(ctx.write, "Hello, {}!", *self.name.borrow()))) } }
# impl cvar::IVisit for Foo { fn visit(&self, f: &mut FnMut(cvar::Node)) { use cvar::{Property, Action}; f(From::from(&Property::new("int", "int description", &self.int, 42))); f(From::from(&Property::new("name", "name description", &self.name, "Casper"))); f(From::from(&Action::new("greet!", "action description", |ctx| self.greet(ctx)))); } }
# struct Root { foo: Foo, } impl cvar::IVisit for Root { fn visit(&self, f: &mut FnMut(cvar::Node)) { use cvar::List; f(From::from(&List::new("foo", "foo description", &self.foo))); } }
# let root = Root { foo: Foo { int: Cell::new(13), name: RefCell::new(String::new()), }, };
assert_eq!(cvar::console::get(&root, "foo.int").unwrap(), "13");

cvar::console::set(&root, "foo.int", "7").unwrap();
assert_eq!(root.foo.int.get(), 7);

cvar::console::reset(&root, "foo.name").unwrap();
assert_eq!(*root.foo.name.borrow(), "Casper");

let mut console = Vec::new();
cvar::console::invoke(&root, "foo.greet!", &mut cvar::Context::new("-o arg", &mut console)).unwrap();
assert_eq!(console, b"Hello, Casper!\n");
```

And use various console functions to interact with the resulting configuration.

See `examples/repl.rs` for a more complex example!
*/

#[cfg(test)]
#[macro_use]
extern crate matches;

use ::std::{io, fmt};
use ::std::str::FromStr;
use ::std::cell::{Cell, RefCell};
use ::std::string::ToString;
use ::std::borrow::Borrow;
use ::std::error::Error as StdError;

pub mod console;

//----------------------------------------------------------------

/// Identifiers are node names joined with a separator.
///
/// Eg. `foo.bar` is an identifier where `foo` and `bar` are names and the `.` is the separator.
///
/// Nodes are allowed to have the separator in their names, creating pseudo hierarchies. No implicit list nodes are created.
///
/// Note: The separator shall be a printable ascii character enforced by debug assert.
pub const JOINER: u8 = b'.';

/// Node interface.
pub trait INode {
	/// Returns the node name.
	fn name(&self) -> &str;
	/// Returns the node description.
	fn description(&self) -> &str;
}

/// Pass through dummy.
///
/// Implements default callbacks for `OnChange` and `OnInvoke`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Pass;

//----------------------------------------------------------------

/// Result with boxed error.
pub type BoxResult<T> = Result<T, Box<StdError>>;

/// Contextless error.
#[derive(Debug)]
pub enum InnerError {
	/// Name not found error.
	///
	/// When traversing the cvar hierarhcy, a child by the name was not found.
	NameError,
	/// Node is not a list.
	///
	/// When traversing the cvar hierarchy, expected the child to implement `IList`.
	///
	/// This happens when an id is given (eg. `foo.prop.baz`) but `foo.prop` is not a list of cvars itself.
	ListError,
	/// Node is not a property.
	///
	/// When traversing the cvar hierarchy, expected the child to implement `IProperty`.
	///
	/// This happens when an id is given (eg. `foo.list`) to get or set its value but `foo.list` is not a property.
	PropError,
	/// Node is not an action.
	///
	/// When traversing the cvar hierarchy, expected the child to implement `IAction`.
	///
	/// This happens when an id is invoked (eg. `foo.bar`) but `foo.bar` is not an action.
	ActionError,
	/// Error parsing the value.
	ParseError(Box<StdError>),
	/// Error validating the value.
	ChangeError(Box<StdError>),
	/// Cannot modify the cvar.
	///
	/// The property is read-only and cannot be modified.
	ConstError,
	/// Error invoking the action.
	InvokeError(Box<StdError>),
}
impl fmt::Display for InnerError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		try!(self.description().fmt(f));
		match *self {
			InnerError::ParseError(ref err) |
			InnerError::ChangeError(ref err) |
			InnerError::InvokeError(ref err) => {
				write!(f, ": {}", err)
			},
			_ => Ok(()),
		}
	}
}
impl StdError for InnerError {
	fn description(&self) -> &str {
		match *self {
			InnerError::NameError => "name not found",
			InnerError::PropError => "property expected",
			InnerError::ListError => "list expected",
			InnerError::ActionError => "action expected",
			InnerError::ParseError(_) => "parse error",
			InnerError::ChangeError(_) => "change error",
			InnerError::ConstError => "property is read-only",
			InnerError::InvokeError(_) => "invoke error",
		}
	}
	fn cause(&self) -> Option<&StdError> {
		match *self {
			InnerError::ParseError(ref err) |
			InnerError::ChangeError(ref err) |
			InnerError::InvokeError(ref err) => {
				Some(&**err)
			},
			_ => None,
		}
	}
}

/// Contextual error.
#[derive(Debug)]
pub struct Error<'a> {
	/// Identifier argument.
	pub id: &'a str,
	/// Specific node that triggered the error, this is a substring of `id`.
	pub name: &'a str,
	/// The actual error.
	pub inner: InnerError,
	_private: (),
}
impl<'a> Error<'a> {
	pub fn new(id: &'a str, name: &'a str, inner: InnerError) -> Error<'a> {
		Error {
			id: id,
			name: name,
			inner: inner,
			_private: (),
		}
	}
}
impl<'a> fmt::Display for Error<'a> {
	fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
		unimplemented!()
	}
}
impl<'a> StdError for Error<'a> {
	fn description(&self) -> &str {
		self.inner.description()
	}
	fn cause(&self) -> Option<&StdError> {
		self.inner.cause()
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
	///
	/// May fail with `InnerError::ParseError` if parsing the value yields an error.
	///
	/// May fail with `InnerError::ChangeError` if validating the value yields an error.
	fn set(&self, val: &str) -> Result<(), InnerError>;
	/// Resets the value to its default.
	fn reset(&self);
	/// Gets the default value as a string.
	fn default(&self) -> String;
	/// Returns the state of the property.
	fn state(&self) -> PropState;
}
impl<'a> fmt::Debug for &'a IProperty {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Property")
			.field("name", &self.name())
			.field("desc", &self.description())
			.field("value", &self.get())
			.field("default", &self.default())
			.field("state", &self.state())
			.finish()
	}
}
impl<'a> From<&'a IProperty> for Node<'a> {
	fn from(prop: &'a IProperty) -> Node<'a> {
		Node::Prop(prop)
	}
}

//----------------------------------------------------------------

/// Abstraction over interior mutability.
pub trait Variable<T> where T: fmt::Debug {
	/// Gets a clone of the value.
	fn get(&self) -> T;
	/// Sets a new value.
	fn set(&self, val: T);
	/// Work with the value without a potentially expensive clone.
	fn with<R, F>(&self, f: F) -> R where F: FnOnce(&T) -> R;
}
impl<'a, T, V> Variable<T> for &'a V where T: fmt::Debug, V: Variable<T> {
	fn get(&self) -> T {
		Variable::get(*self)
	}
	fn set(&self, val: T) {
		Variable::set(*self, val)
	}
	fn with<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
		Variable::with(*self, f)
	}
}
impl<T> Variable<T> for Cell<T> where T: Copy + fmt::Debug {
	fn get(&self) -> T {
		Cell::get(self)
	}
	fn set(&self, val: T) {
		Cell::set(self, val)
	}
	fn with<R, F>(&self, f: F) -> R where F: FnOnce(&T) -> R {
		(f)(&Cell::get(self))
	}
}
impl<T> Variable<T> for RefCell<T> where T: Clone + fmt::Debug {
	fn get(&self) -> T {
		self.borrow().clone()
	}
	fn set(&self, val: T) {
		*self.borrow_mut() = val;
	}
	fn with<R, F>(&self, f: F) -> R where F: FnOnce(&T) -> R {
		(f)(&*self.borrow())
	}
}

//----------------------------------------------------------------

/// Accepted property value types.
///
/// Functionality is duplicated to allow custom implementations for external types, eg. `Option<T>`.
pub trait Value: Clone + PartialEq + fmt::Debug {
	fn parse(val: &str) -> BoxResult<Self>;
	fn to_string(&self) -> String;
}

/// Implement [`Value`](trait.Value.html) automatically for types that have appropriate `FromStr` and `ToString` implementations.
pub trait AutoValue: Copy + PartialEq + FromStr + ToString + fmt::Debug
	where Self::Err: 'static + StdError {}

impl AutoValue for i8 {}
impl AutoValue for i16 {}
impl AutoValue for i32 {}
impl AutoValue for i64 {}
impl AutoValue for isize {}
impl AutoValue for u8 {}
impl AutoValue for u16 {}
impl AutoValue for u32 {}
impl AutoValue for u64 {}
impl AutoValue for usize {}
impl AutoValue for f32 {}
impl AutoValue for f64 {}
impl AutoValue for bool {}

impl<T> Value for T
	where T: AutoValue,
	      T::Err: 'static + StdError
{
	fn parse(val: &str) -> BoxResult<Self> {
		Ok(try!(val.parse()))
	}
	fn to_string(&self) -> String {
		ToString::to_string(self)
	}
}

impl<T> Value for Option<T>
	where T: AutoValue,
	      T::Err: 'static + StdError
{
	fn parse(val: &str) -> BoxResult<Self> {
		if val == "None" {
			Ok(None)
		}
		else {
			Ok(Some(try!(val.parse())))
		}
	}
	fn to_string(&self) -> String {
		match *self {
			Some(ref val) => ToString::to_string(val),
			None => String::from("None"),
		}
	}
}

impl Value for String {
	fn parse(val: &str) -> BoxResult<String> {
		Ok(String::from(val))
	}
	fn to_string(&self) -> String {
		self.clone()
	}
}

//----------------------------------------------------------------

/// Property callback when its value is changed.
pub trait OnChange<T> {
	/// Given the old and assigned values produces the new value.
	///
	/// May return a validation error.
	fn change(&self, old: &T, val: T) -> BoxResult<T>;
}
impl<T> OnChange<T> for Pass {
	fn change(&self, _: &T, val: T) -> BoxResult<T> {
		Ok(val)
	}
}
impl<T, F> OnChange<T> for F where F: Fn(&T, T) -> BoxResult<T> {
	fn change(&self, old: &T, val: T) -> BoxResult<T> {
		(self)(old, val)
	}
}

//----------------------------------------------------------------

/// Property instance.
///
/// The `N`ame and `D`escription types allow abstracting over `&'static str`, `&'a str` and `String`. This supports dynamic cvars while only paying for what you need.
///
/// The `V`ariable type holds a [value](trait.Value.html) of the underlying `T`ype with interior mutability.
///
/// `F` is the callable type called when the value is changed.
pub struct Property<N, D, T, V, F>
	where N: Borrow<str>,
	      D: Borrow<str>,
	      T: Value,
	      V: Variable<T>,
	      F: OnChange<T>
{
	name: N,
	desc: D,
	var: V,
	def: T,
	change: F,
}
impl<N, D, T, V> Property<N, D, T, V, Pass> where N: Borrow<str>, D: Borrow<str>, T: Value, V: Variable<T> {
	/// Creates a new `Property`.
	///
	/// Given a name, description, [variable](trait.Variable.html) and default.
	///
	/// ```
	/// // The underlying data wrapped in a `Cell`.
	/// use std::cell::Cell;
	/// let var = Cell::new(13);
	///
	/// // The variable wrapped in a `Property`.
	/// use cvar::{Property, IProperty};
	/// let prop = Property::new("prop", "property description", &var, 42);
	/// assert_eq!(prop.get(), "13");
	/// prop.reset();
	/// assert_eq!(var.get(), 42);
	/// ```
	pub fn new<I>(name: N, desc: D, var: V, def: I) -> Property<N, D, T, V, Pass> where I: Into<T> {
		Property {
			name: name,
			desc: desc,
			var: var,
			def: def.into(),
			change: Pass,
		}
	}
}
impl<N, D, T, V> Property<N, D, T, V, Pass> where N: Borrow<str>, D: Borrow<str>, T: Value, V: Variable<T> {
	/// Creates a new `Property` with [change](trait.OnChange.html) callback.
	///
	/// Called when a new value is assigned to the property through the `set` and `reset` methods.
	/// It does not monitor the `V`older for changes.
	///
	/// The default value must always validate successfully or `reset` will panic.
	pub fn change<F>(self, change: F) -> Property<N, D, T, V, F> where F: Fn(&T, T) -> BoxResult<T> {
		debug_assert!(self.var.with(|old| (change)(old, self.def.clone()).is_ok()), "default value did not validate");
		Property {
			name: self.name,
			desc: self.desc,
			var: self.var,
			def: self.def,
			change: change,
		}
	}
}
impl<N, D, T, V, F> INode for Property<N, D, T, V, F> where N: Borrow<str>, D: Borrow<str>, T: Value, V: Variable<T>, F: OnChange<T> {
	fn name(&self) -> &str {
		self.name.borrow()
	}
	fn description(&self) -> &str {
		self.desc.borrow()
	}
}
impl<N, D, T, V, F> IProperty for Property<N, D, T, V, F> where N: Borrow<str>, D: Borrow<str>, T: Value, V: Variable<T>, F: OnChange<T> {
	fn get(&self) -> String {
		self.var.get().to_string()
	}
	fn set(&self, val: &str) -> Result<(), InnerError> {
		match Value::parse(val) {
			Ok(val) => {
				match self.var.with(|old| self.change.change(old, val)) {
					Ok(val) => self.var.set(val),
					Err(err) => return Err(InnerError::ChangeError(err)),
				}
			},
			Err(err) => return Err(InnerError::ParseError(err)),
		};
		Ok(())
	}
	fn reset(&self) {
		let val = self.var.with(|old| self.change.change(old, self.def.clone())).unwrap();
		self.var.set(val);
	}
	fn default(&self) -> String {
		self.def.to_string()
	}
	fn state(&self) -> PropState {
		match self.var.with(|val| val == &self.def) {
			true => PropState::Default,
			false => PropState::UserSet,
		}
	}
}
impl<'s, N, D, T, V, F> From<&'s Property<N, D, T, V, F>> for Node<'s> where N: Borrow<str>, D: Borrow<str>, T: Value, V: Variable<T>, F: OnChange<T> {
	fn from(prop: &'s Property<N, D, T, V, F>) -> Node<'s> {
		Node::Prop(prop)
	}
}

//----------------------------------------------------------------

/// Node interface.
#[derive(Copy, Clone)]
pub enum Node<'a> {
	Prop(&'a IProperty),
	List(&'a IList),
	Action(&'a IAction),
}
impl<'a> INode for Node<'a> {
	fn name(&self) -> &str {
		match *self {
			Node::Prop(prop) => prop.name(),
			Node::List(list) => list.name(),
			Node::Action(act) => act.name(),
		}
	}
	fn description(&self) -> &str {
		match *self {
			Node::Prop(prop) => prop.description(),
			Node::List(list) => list.description(),
			Node::Action(act) => act.description(),
		}
	}
}
impl<'a> fmt::Debug for Node<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Node::Prop(prop) => {
				prop.fmt(f)
			},
			Node::List(list) => {
				list.fmt(f)
			},
			Node::Action(act) => {
				act.fmt(f)
			},
		}
	}
}

/// Visitor Pattern interface.
pub trait IVisit {
	/// Calls the callback `f` with every child casted as `Node`.
	///
	/// ```
	/// use ::std::cell::Cell;
	/// struct Object {
	/// 	foo: Cell<i32>,
	/// 	bar: Cell<i32>,
	/// }
	/// impl cvar::IVisit for Object {
	/// 	fn visit(&self, f: &mut FnMut(cvar::Node)) {
	/// 		use cvar::{Property};
	/// 		f(From::from(&Property::new("foo", "foo description", &self.foo, 42)));
	/// 		f(From::from(&Property::new("bar", "bar description", &self.bar, 12)));
	/// 	}
	/// }
	/// ```
	fn visit(&self, f: &mut FnMut(Node));
}
impl<'a> fmt::Debug for &'a IVisit {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut f = f.debug_list();
		self.visit(&mut |node| {
			f.entry(&node);
		});
		f.finish()
	}
}

/// List node interface.
///
/// Provides an object safe interface for lists, type erasing its implementation.
pub trait IList: INode {
	/// Returns the visitor interface to access its children.
	fn as_visit(&self) -> &IVisit;
}
impl<'a> fmt::Debug for &'a IList {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("List")
			.field("name", &self.name())
			.field("desc", &self.description())
			.field("children", &self.as_visit())
			.finish()
	}
}
impl<'a> From<&'a IList> for Node<'a> {
	fn from(list: &'a IList) -> Node<'a> {
		Node::List(list)
	}
}

//----------------------------------------------------------------

/// List instance.
///
/// The `N`ame and `D`escription types allow abstracting over `&'static str`, `&'a str` and `String`. This supports dynamic cvars while only paying for what you need.
pub struct List<'a, N, D>
	where N: Borrow<str>,
	      D: Borrow<str>
{
	name: N,
	desc: D,
	visit: &'a IVisit,
}
impl<'a, N, D> List<'a, N, D> where N: Borrow<str>, D: Borrow<str> {
	/// Creates a new `List`.
	///
	/// Given a name, description and [visitor](trait.IVisit.html) to access its children.
	pub fn new(name: N, desc: D, visit: &'a IVisit) -> List<'a, N, D> {
		List {
			name: name,
			desc: desc,
			visit: visit,
		}
	}
}
impl<'a, N, D> INode for List<'a, N, D> where N: Borrow<str>, D: Borrow<str> {
	fn name(&self) -> &str {
		self.name.borrow()
	}
	fn description(&self) -> &str {
		self.desc.borrow()
	}
}
impl<'a, N, D> IList for List<'a, N, D> where N: Borrow<str>, D: Borrow<str> {
	fn as_visit(&self) -> &IVisit {
		self.visit
	}
}
impl<'s, 'a, N, D> From<&'s List<'a, N, D>> for Node<'s> where N: Borrow<str>, D: Borrow<str> {
	fn from(val: &'s List<'a, N, D>) -> Node<'s> {
		Node::List(val)
	}
}

//----------------------------------------------------------------

/// Invocation context.
///
/// Provides a place to pass parameters through to the action callback.
pub struct Context<'a> {
	/// The command arguments.
	///
	/// There are no extra constraints on the formatting, it is passed through to the underlying action.
	pub args: &'a str,
	/// A console-like interface.
	///
	/// Allows the action to let the world know what it has to say.
	pub write: &'a mut io::Write,
	_private: (),
}
impl<'a> Context<'a> {
	/// Constructs a new invocation context.
	pub fn new(args: &'a str, write: &'a mut io::Write) -> Context<'a> {
		Context {
			args: args,
			write: write,
			_private: (),
		}
	}
}

/// Action node interface.
///
/// Provides an object safe interface for actions, type erasing its implementation.
pub trait IAction: INode {
	/// Invoke the callback associated with the Action.
	fn invoke(&self, ctx: &mut Context) -> Result<(), InnerError>;
}
impl<'a> fmt::Debug for &'a IAction {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Action")
			.field("name", &self.name())
			.field("desc", &self.description())
			.finish()
	}
}
impl<'a> From<&'a IAction> for Node<'a> {
	fn from(act: &'a IAction) -> Node<'a> {
		Node::Action(act)
	}
}

/// Action callback when invoked.
pub trait OnInvoke {
	fn invoke(&self, ctx: &mut Context) -> BoxResult<()>;
}
impl OnInvoke for Pass {
	fn invoke(&self, _: &mut Context) -> BoxResult<()> {
		Ok(())
	}
}
impl<F> OnInvoke for F where F: Fn(&mut Context) -> BoxResult<()> {
	fn invoke(&self, ctx: &mut Context) -> BoxResult<()> {
		(self)(ctx)
	}
}

//----------------------------------------------------------------

/// Action instance.
///
/// The `N`ame and `D`escription types allow abstracting over `&'static str`, `&'a str` and `String`. This supports dynamic cvars while only paying for what you need.
///
/// `F` is the callable type called when the action is invoked.
pub struct Action<N, D, F>
	where N: Borrow<str>,
	      D: Borrow<str>,
	      F: OnInvoke
{
	name: N,
	desc: D,
	f: F,
}
impl<N, D> Action<N, D, Pass> where N: Borrow<str>, D: Borrow<str> {
	/// Creates a new `Action`.
	///
	/// Given a name, description and a [callback](trait.OnInvoke.html) to be invoked.
	pub fn new<F>(name: N, desc: D, f: F) -> Action<N, D, F> where F: Fn(&mut Context) -> BoxResult<()> {
		Action {
			name: name,
			desc: desc,
			f: f,
		}
	}
}
impl<N, D, F> INode for Action<N, D, F> where N: Borrow<str>, D: Borrow<str>, F: OnInvoke {
	fn name(&self) -> &str {
		self.name.borrow()
	}
	fn description(&self) -> &str {
		self.desc.borrow()
	}
}
impl<N, D, F> IAction for Action<N, D, F> where N: Borrow<str>, D: Borrow<str>, F: OnInvoke {
	fn invoke(&self, ctx: &mut Context) -> Result<(), InnerError> {
		match self.f.invoke(ctx) {
			Ok(ok) => Ok(ok),
			Err(err) => Err(InnerError::InvokeError(err)),
		}
	}
}
impl<'s, N, D, F> From<&'s Action<N, D, F>> for Node<'s> where N: Borrow<str>, D: Borrow<str>, F: OnInvoke {
	fn from(val: &'s Action<N, D, F>) -> Node<'s> {
		Node::Action(val)
	}
}
