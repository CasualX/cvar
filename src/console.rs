/*!
Interact with the cvar hierarchy.

The design of this library makes recursive depth-first pre-order traversal the only feasable method to walk the cvar hierarchy.

This trade-off allows the hierarchy to be constructed lazily with very convenient stack-allocated resources.
*/

use super::*;

//----------------------------------------------------------------

/// Sets a property its value.
#[inline]
pub fn set<'a>(root: &IVisit, id: &'a str, val: &'a str) -> Result<(), Error<'a>> {
	find(root, id, |node| {
		if let Node::Prop(prop) = node {
			prop.set(val)
		}
		else {
			Err(InnerError::PropError)
		}
	})
}

/// Gets a property its value.
#[inline]
pub fn get<'a>(root: &IVisit, id: &'a str) -> Result<String, Error<'a>> {
	find(root, id, |node| {
		if let Node::Prop(prop) = node {
			Ok(prop.get())
		}
		else {
			Err(InnerError::PropError)
		}
	})
}

/// Resets properties to their default.
///
/// Given a list node will reset all its children to their default. Ignores action nodes.
pub fn reset<'a>(root: &IVisit, id: &'a str) -> Result<(), Error<'a>> {
	find(root, id, |node| {
		reset_rec(node);
		Ok(())
	})
}
/// Resets all properties to their default.
pub fn reset_all(root: &IVisit) {
	visit(root, |node| {
		reset_rec(node);
	});
}
fn reset_rec<'a>(node: Node<'a>) {
	match node {
		Node::Prop(prop) => {
			prop.reset();
		},
		Node::List(list) => {
			visit(list.as_visit(), |node| {
				reset_rec(node);
			});
		},
		Node::Action(_) => {},
	};
}

/// Gets a property its default.
#[inline]
pub fn default<'a>(root: &IVisit, id: &'a str) -> Result<String, Error<'a>> {
	find(root, id, |node| {
		if let Node::Prop(prop) = node {
			Ok(prop.default())
		}
		else {
			Err(InnerError::PropError)
		}
	})
}

/// Gets a property its state.
pub fn state<'a>(root: &IVisit, id: &'a str) -> Result<PropState, Error<'a>> {
	find(root, id, |node| {
		if let Node::Prop(prop) = node {
			Ok(prop.state())
		}
		else {
			Err(InnerError::PropError)
		}
	})
}

//----------------------------------------------------------------

/// Compares an identifier to a node name.
///
/// * `CompareId::True` if they compare equal.
/// * `CompareId::Part` if the id starts with name and is followed by the `JOINER`.
/// * `CompareId::False` in all other cases.
///
/// Node names are allowed to contain `JOINER`, eg: `Action::new("foo.bar", "desc", |ctx| Ok(()))`.
/// This does not create a `foo` list node, but merely allows the node to pretend to be part of one.
///
/// Node names are allowed to be the empty string. It works exactly the same as any other name.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum CompareId<'a> {
	False,
	True,
	Part(&'a str),
}
impl<'a> CompareId<'a> {
	fn cmp(id: &'a str, name: &str) -> CompareId<'a> {
		// Let's work with byte slices for convenience.
		let id = id.as_bytes();
		let name = name.as_bytes();
		if id.starts_with(name) {
			// When the `id` starts with `name` check what follows immediately after.
			match id.get(name.len()) {
				Some(&JOINER) => CompareId::Part(unsafe {
					use ::std::str::from_utf8_unchecked;
					use ::std::slice::from_raw_parts;
					// Split right after the `JOINER` matched earlier
					let split_at = name.len().wrapping_add(1);
					// Create a slice starting from here until the end
					from_utf8_unchecked(from_raw_parts(id.as_ptr().offset(split_at as isize), id.len().wrapping_sub(split_at)))
					// Why not `id.split_at(name.len() + 1).1`? :)
				}),
				Some(_) => CompareId::False,
				None => CompareId::True,
			}
		}
		else {
			CompareId::False
		}
	}
}

/// Finds a cvar by its node identifier and invokes the callback with it.
///
/// Should not return `Err(InnerError::NameError)` from the callback.
#[inline]
pub fn find<'a, F, R>(root: &IVisit, id: &'a str, mut f: F) -> Result<R, Error<'a>>
	where F: FnMut(Node) -> Result<R, InnerError>
{
	// This comment below was written when the code read:
	//
	// let mut ok_val = mem::uninitialized();
	//
	// I've changed it to use an `Option<T>` initialized to `None`.
	// This incurs some overhead which can be fixed with `union` when it hits stable.

	// For reasons, `find_rec` cannot be templated, his makes it tricky to return `Ok(R)` values.
	// Despite being `FnMut`, `f` **will** be called at most once making it safe to hoist the return value out of the closure.
	//
	// * The closure is called and returns `Ok(R)` then `ok_val` is initialized and `find_rec` returns `Ok(())`.
	// * The closure is called and returns `Err(InnerError)` then `ok_val` is uninitialized and `find_rec` returns `Err(Error)`.
	// * The closure is not called and `find_rec` returns `Err(Error)` (name error).
	//
	// Just make sure to `mem::forget` it on the error case.
	//
	// FIXME! Unwinding will cause the destructor to run...
	//        Fixed when `union` lands in stable: https://github.com/rust-lang/rfcs/pull/197
	use ::std::{mem};
	let mut ok_val: Option<R> = None;
	let result = find_rec(root, id, id, &mut |node| {
		mem::forget(mem::replace(&mut ok_val, Some(try!((f)(node)))));
		Ok(())
	});
	// Replace the `ok_val` in the result
	result.map(|_| ok_val.unwrap())
}
fn find_rec<'a>(list: &IVisit, target: &'a str, id: &'a str, f: &mut FnMut(Node) -> Result<(), InnerError>) -> Result<(), Error<'a>> {
	let mut result = Err(Error::new(target, id, InnerError::NameError));
	list.visit(&mut |node| {
		// Keep looking as long as we get name errors
		let name_error = match result {
			Err(Error { inner: InnerError::NameError, .. }) => true,
			_ => false,
		};
		if name_error {
			let name = node.name();
			let val = match CompareId::cmp(id, name) {
				CompareId::True => {
					f(node).map_err(|err| Error::new(target, id, err))
				},
				CompareId::Part(tail) => {
					if let Node::List(list) = node {
						find_rec(list.as_visit(), target, tail, f)
					}
					else {
						let sub_id = unsafe {
							use ::std::str::from_utf8_unchecked;
							use ::std::slice::from_raw_parts;
							from_utf8_unchecked(from_raw_parts(id.as_ptr(), name.len()))
						};
						Err(Error::new(target, sub_id, InnerError::ListError))
					}
				},
				CompareId::False => {
					return;
				},
			};
			use ::std::mem;
			mem::forget(mem::replace(&mut result, val));
		}
	});
	result
}

/// Visits all nodes in the cvar hierarchy and invokes the callback with each `Node`.
///
/// The context for the parent nodes is unavailable, see [`walk()`](fn.walk.html).
/// This makes it not possible to reconstruct the node identifier.
#[inline]
pub fn visit<F>(root: &IVisit, mut f: F) where F: FnMut(Node) {
	visit_rec(root, &mut f);
}
fn visit_rec<F>(list: &IVisit, f: &mut F) where F: FnMut(Node) {
	list.visit(&mut |node| {
		f(node);
		if let Node::List(list) = node {
			visit_rec(list.as_visit(), f);
		};
	});
}

#[derive(Copy, Clone)]
struct WalkList<'a> {
	piece: &'a IList,
	up: Option<&'a WalkList<'a>>,
}
/// Iterator over parent list nodes.
#[derive(Copy, Clone)]
pub struct Walk<'a>(Option<&'a WalkList<'a>>);
impl<'a> Walk<'a> {
	/// Builds a complete identifier string.
	///
	/// Given the top-level node name and a chain of parents will build the fully expanded node identifier.
	pub fn id(self, name: &str) -> String {
		// The parent list is in the reverse order, oh noes whatever shall we do!
		debug_assert!(JOINER >= b' ' && JOINER < 127, "`JOINER` must be ascii");

		// Start by counting how many bytes we'll need for the parent names + dot characters
		let num_bytes = self.fold(name.len(), |acc, it| acc + it.name().len() + 1);

		// Allocate that much memory
		let mut id = Vec::with_capacity(num_bytes);
		unsafe {
			use ::std::slice::from_raw_parts_mut;

			// Going to write this backwards starting at the end
			let mut write_ptr = id.as_mut_slice().as_mut_ptr().offset(num_bytes as isize);

			// Start by writing the node name
			write_ptr = write_ptr.offset(-(name.len() as isize));
			from_raw_parts_mut(write_ptr, name.len()).copy_from_slice(name.as_bytes());

			// Now write the parent names
			for parent in self {
				let parent_name = parent.name();
				write_ptr = write_ptr.offset(-1);
				*write_ptr = JOINER;
				write_ptr = write_ptr.offset(-(parent_name.len() as isize));
				from_raw_parts_mut(write_ptr, parent_name.len()).copy_from_slice(parent_name.as_bytes());
			}

			// Do some sanity checking and make it visible
			// As the inputs are valid utf-8 and `JOINER` is ascii means the final result is also valid utf-8
			debug_assert_eq!(write_ptr, id.as_mut_slice().as_mut_ptr());
			id.set_len(num_bytes);
			String::from_utf8_unchecked(id)
		}
	}
}
impl<'a> From<Option<&'a WalkList<'a>>> for Walk<'a> {
	fn from(walk: Option<&'a WalkList<'a>>) -> Walk<'a> {
		Walk(walk)
	}
}
impl<'a> Iterator for Walk<'a> {
	type Item = &'a IList;
	fn next(&mut self) -> Option<&'a IList> {
		let next = self.0;
		match next {
			Some(it) => {
				let piece = it.piece;
				self.0 = it.up;
				Some(piece)
			},
			None => {
				None
			},
		}
	}
}

/// Walks the cvar hierarchy while keeping track of the parent context.
///
/// The callback `f` accepts an iterable `Walk` context to retrieve the parent list nodes and a `Node`.
#[inline]
pub fn walk<F>(root: &IVisit, mut f: F) where F: FnMut(Walk, Node) {
	walk_rec(root, None, &mut f)
}
fn walk_rec<F>(list: &IVisit, walk: Option<&WalkList>, f: &mut F) where F: FnMut(Walk, Node) {
	list.visit(&mut |node| {
		f(walk.into(), node);
		if let Node::List(list) = node {
			let walk = WalkList {
				piece: list,
				up: walk,
			};
			walk_rec(list.as_visit(), Some(&walk), f);
		}
	});
}

//----------------------------------------------------------------

/// Invokes an action.
#[inline]
pub fn invoke<'a>(root: &IVisit, id: &'a str, ctx: &mut Context) -> Result<(), Error<'a>> {
	find(root, id, |node| {
		if let Node::Action(act) = node {
			act.invoke(ctx)
		}
		else {
			Err(InnerError::PropError)
		}
	})
}

//----------------------------------------------------------------

#[cfg(test)]
mod tests {
use super::{CompareId};
use super::{WalkList};
use super::*;
use super::super::*;
#[test]
fn compare_id() {
	assert_eq!(CompareId::cmp("foo", "foo"), CompareId::True);
	assert_eq!(CompareId::cmp("foo.bar", "foo"), CompareId::Part("bar"));
	assert_eq!(CompareId::cmp("foo.bar", "foo.bar"), CompareId::True);
	assert_eq!(CompareId::cmp("foo.bar.baz", "foo.bar"), CompareId::Part("baz"));
	assert_eq!(CompareId::cmp("fooz", "foo"), CompareId::False);
	assert_eq!(CompareId::cmp("foo.bar.baz", "bar.baz"), CompareId::False);
	assert_eq!(CompareId::cmp("foo", "foo.bar"), CompareId::False);
	// Degenerate cases: empty string node names
	assert_eq!(CompareId::cmp("foo.", "foo"), CompareId::Part(""));
	assert_eq!(CompareId::cmp("foo.", "foo."), CompareId::True);
	assert_eq!(CompareId::cmp("foo", "foo."), CompareId::False);
	assert_eq!(CompareId::cmp("", ""), CompareId::True);
	assert_eq!(CompareId::cmp(".", ""), CompareId::Part(""));
	assert_eq!(CompareId::cmp(".", "."), CompareId::True);
	assert_eq!(CompareId::cmp("", "."), CompareId::False);
	assert_eq!(CompareId::cmp(".foo", "foo"), CompareId::False);
	assert_eq!(CompareId::cmp(".foo", ".foo"), CompareId::True);
	assert_eq!(CompareId::cmp("foo", ".foo"), CompareId::False);
}
#[test]
fn build_id() {
	// Mock some walk structures
	struct MyList<'a> { name: &'a str, }
	impl<'a> INode for MyList<'a> {
		fn name(&self) -> &str { self.name }
		fn description(&self) -> &str { "desc" }
	}
	impl<'a> IList for MyList<'a> {
		fn as_visit(&self) -> &IVisit { self }
	}
	impl<'a> IVisit for MyList<'a> {
		fn visit(&self, _: &mut FnMut(Node)) {}
	}
	let l0 = MyList { name: "list" };
	let l1 = MyList { name: "foo" };
	let l2 = MyList { name: "bar" };
	let n0 = WalkList { piece: &l0, up: None };
	let n1 = WalkList { piece: &l1, up: Some(&n0) };
	let n2 = WalkList { piece: &l2, up: Some(&n1) };
	let walk: Walk = Some(&n2).into();
	assert_eq!(walk.id("child"), "list.foo.bar.child");
	assert_eq!(Walk(None).id("toplevel"), "toplevel");
}
#[test]
#[should_panic = "panicked safely"]
fn find_dropck() {
	struct Root;
	impl IVisit for Root {
		fn visit(&self, f: &mut FnMut(Node)) {
			f(From::from(&Property::new("panic", "panic safe", ::std::cell::Cell::new(0), 0)));
		}
	}
	// When an error is triggerd, must certainly *not* drop that uninitialized result
	#[derive(Debug)]
	struct PanicOnDrop;
	impl Drop for PanicOnDrop { fn drop(&mut self) { panic!("crashed"); } }
	assert_matches!(
		find(&Root, "name error", |_| Ok(PanicOnDrop)),
		Err(Error { inner: InnerError::NameError, .. }));
	// Also should be safe when unwinding
	let _ = find::<_, PanicOnDrop>(&Root, "panic", |_| panic!("panicked safely"));
}
use ::std::cell::{Cell, RefCell};
struct Foo {
	int: Cell<i32>,
	float: Cell<f32>,
	string: RefCell<String>,
}
impl Foo {
	fn action(&self, ctx: &mut Context) -> BoxResult<()> {
		writeln!(ctx.write, "I am {:?} and {}", *self.string.borrow(), ctx.args).ok();
		Ok(())
	}
}
impl IVisit for Foo {
	fn visit(&self, f: &mut FnMut(Node)) {
		f(From::from(&Property::new("int", "int desc", &self.int, 42)));
		f(From::from(&Property::new("float", "float desc", &self.float, 1.2f32)));
		f(From::from(&Property::new("string", "string desc", &self.string, String::new())));
		f(From::from(&Action::new("action", "action desc", |ctx| self.action(ctx))));
	}
}
struct Root {
	before: Cell<i32>,
	foo: Foo,
	after: Cell<i32>,
}
impl IVisit for Root {
	fn visit(&self, f: &mut FnMut(Node)) {
		f(From::from(&Property::new("foo.before", "foo.before desc", &self.before, 1)));
		f(From::from(&List::new("foo", "foo desc", &self.foo)));
		f(From::from(&Property::new("foo.after", "foo.after desc", &self.after, 2)));
	}
}
fn root() -> Root {
	Root {
		before: Cell::new(1),
		foo: Foo {
			int: Cell::new(13),
			float: Cell::new(-0.1f32),
			string: RefCell::new(String::from("groot")),
		},
		after: Cell::new(2),
	}
}
#[test]
fn main() {
	let root = root();
	assert_matches!(set(&root, "foo.float", "-1"), Ok(()));
	assert_eq!(root.foo.float.get(), -1f32);
	assert_matches!(set(&root, "foo.before", "11"), Ok(()));
	assert_matches!(set(&root, "foo.after", "22"), Ok(()));
	assert_matches!(set(&root, "foo.int", "parse error"), Err(Error { name: "int", inner: InnerError::ParseError(_), .. }));
	assert_matches!(set(&root, "foo.list.bar", "name error"), Err(Error { name: "list.bar", inner: InnerError::NameError, .. }));
	assert_matches!(set(&root, "foo.int.bar", "list error"), Err(Error { name: "int", inner: InnerError::ListError, .. }));
	assert_matches!(set(&root, "foo.action.bar", "list error"), Err(Error { name: "action", inner: InnerError::ListError, .. }));
	assert_matches!(set(&root, "foo.action", "prop error"), Err(Error { name: "action", inner: InnerError::PropError, .. }));
}
}
