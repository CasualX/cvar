/*!
Interact with the configuration variables.

The design of this library makes recursive depth-first pre-order traversal the only feasable method to walk the cvars.

This trade-off allows the hierarchy to be constructed lazily with very convenient stack-allocated resources.
*/

use super::*;

/// Pokes the cvar tree.
///
/// Returns `false` if there was an error, the path does not exist or the args were not valid.
///
/// This function combines the behavior of [`get`], [`set`], and [`invoke`].
pub fn poke(root: &mut dyn IVisit, path: &str, args: Option<&str>, writer: &mut dyn IWrite) -> bool {
	let mut result = false;
	if path.len() > 0 {
		if !find(root, path, |node| {
			match node.as_node() {
				Node::Prop(prop) => {
					if let Some(val) = args {
						let mut err = String::new();
						if prop.set(val, &mut err) {
							let value = prop.get_value().to_string();
							// cvar.prop is "true"
							let _ = writeln!(writer, "{path} is {value:?}");
							result = true;
						}
						else {
							// error: cvar.prop "true": not a number
							let _ = writeln!(writer, "error: {path} {val:?}: {err}");
						}
					}
					else {
						let value = prop.get_value().to_string();
						// cvar.prop is "true"
						let _ = writeln!(writer, "{path} is {value:?}");
						result = true;
					}
				},
				Node::List(list) => {
					_print_nodes(list.as_ivisit(), Some(path), writer);
					result = true;
				},
				Node::Action(act) => {
					act.invoke(args.unwrap_or(""), writer);
					result = true;
				},
			}
		}) {
			let _ = writeln!(writer, "unknown: {path}");
		}
	}
	else {
		_print_nodes(root, None, writer);
		result = true;
	}
	result
}

fn _print_node(node: &mut dyn INode, path: Option<&str>, writer: &mut dyn IWrite) -> fmt::Result {
	if let Some(path) = path {
		writer.write_str(path)?;
		writer.write_str(".")?;
	}
	match node.as_node() {
		Node::Prop(prop) => {
			let value = prop.get_value().to_string();
			let name = prop.name();
			write!(writer, "{name} is {value:?}")?;
		},
		Node::List(list) => {
			let name = list.name();
			writer.write_str(name)?;
			writer.write_str("...")?;
		},
		Node::Action(act) => {
			let name = act.name();
			writer.write_str(name)?;
		},
	}
	writer.write_str("\n")?;
	Ok(())
}
fn _print_nodes(root: &mut dyn IVisit, path: Option<&str>, writer: &mut dyn IWrite) {
	root.visit(&mut move |node| {
		let _ = _print_node(node, path, writer);
	});
}

//----------------------------------------------------------------

/// Sets a property's value parsed from a string.
///
/// If the path is an action it is invoked with the value as the argument.
#[inline]
pub fn set(root: &mut dyn IVisit, path: &str, val: &str, writer: &mut dyn IWrite) -> bool {
	let mut result = false;
	if !find(root, path, |node| {
		match node.as_node() {
			Node::Prop(prop) => {
				let mut err = String::new();
				if prop.set(val, &mut err) {
					result = true;
				}
				else {
					// error: cvar.prop "true": not a number
					let _ = writeln!(writer, "error: {path} {val:?}: {err}");
				}
			},
			Node::List(_) => {},
			Node::Action(act) => {
				act.invoke(val, writer);
			},
		}
	}) {
		let _ = writeln!(writer, "unknown: {path}");
	}
	result
}

/// Gets a property's value as a string.
///
/// Returns `None` if the path does not lead to a property.
#[inline]
pub fn get(root: &mut dyn IVisit, path: &str) -> Option<String> {
	let mut result = None;
	find(root, path, |node| {
		if let Node::Prop(prop) = node.as_node() {
			result = Some(prop.get_value().to_string());
		}
	});
	result
}

/// Sets a property's value directly.
///
/// If the path is an action it is invoked with the value as the argument.
#[inline]
pub fn set_value(root: &mut dyn IVisit, path: &str, val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
	let mut result = false;
	if !find(root, path, |node| {
		match node.as_node() {
			Node::Prop(prop) => {
				let mut err = String::new();
				if prop.set_value(val, &mut err) {
					result = true;
				}
				else {
					// error: cvar.prop "true": not a number
					let _ = writeln!(writer, "error: {path} {val:?}: {err}");
				}
			},
			Node::List(_) => {},
			Node::Action(act) => {
				act.invoke(&val.to_string(), writer);
			},
		}
	}) {
		let _ = writeln!(writer, "unknown: {path}");
	}
	result
}

/// Gets a property's value directly.
///
/// Returns `None` if the path does not lead to a property of the expected type.
#[inline]
pub fn get_value<T: Clone + 'static>(root: &mut dyn IVisit, path: &str) -> Option<T> {
	let mut value = None;
	find(root, path, |node| {
		if let Node::Prop(prop) = node.as_node() {
			if let Some(any) = prop.get_value().downcast_ref::<T>() {
				value = Some(any.clone());
			}
		}
	});
	value
}

/// Resets properties to their default.
///
/// Given a list node will reset all its children to their default. Ignores action nodes.
#[inline]
pub fn reset(root: &mut dyn IVisit, path: &str) -> bool {
	find(root, path, |node| {
		match node.as_node() {
			Node::Prop(prop) => prop.reset(),
			Node::List(list) => reset_all(list.as_ivisit()),
			Node::Action(_) => (),
		}
	})
}
/// Resets all properties to their default.
#[inline]
pub fn reset_all(root: &mut dyn IVisit) {
	root.visit(&mut |node| {
		match node.as_node() {
			Node::Prop(prop) => prop.reset(),
			Node::List(list) => reset_all(list.as_ivisit()),
			Node::Action(_) => (),
		}
	});
}

/// Lists all properties and actions in the visitor.
#[inline]
pub fn print(root: &mut dyn IVisit, path: &str, writer: &mut dyn IWrite) {
	if path.len() > 0 {
		if !find(root, path, |node| {
			let _ = _print_node(node, Some(path), writer);
		}) {
			let _ = writeln!(writer, "unknown: {path}");
		}
	}
	else {
		_print_nodes(root, None, writer);
	}
}

//----------------------------------------------------------------

#[inline]
fn split_at<'a>(path: &'a str, index: usize) -> Option<(&str, &u8, &str)> {
	let at = path.as_bytes().get(index)?;
	let prefix = path.get(..index)?;
	let suffix = path.get(index + 1..)?;
	Some((prefix, at, suffix))
}

/// Compares a path to a node name.
///
/// * `ComparePath::True` if they compare equal.
/// * `ComparePath::Part` if the path starts with name and is followed by `.`.
/// * `ComparePath::False` in all other cases.
///
/// Node names are allowed to contain `.`, eg: `Action::new("foo.bar", "desc", |ctx| Ok(()))`.
/// This does not create a `foo` list node, but merely allows the node to pretend to be part of one.
///
/// Node names are allowed to be the empty string, while confusing there's nothing special about it.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ComparePath<'a> {
	False,
	True,
	Part(&'a str),
}
impl<'a> ComparePath<'a> {
	#[inline]
	fn cmp(path: &'a str, name: &str) -> ComparePath<'a> {
		match split_at(path, name.len()) {
			Some((prefix, &b'.', suffix)) => {
				if prefix == name {
					ComparePath::Part(suffix)
				}
				else {
					ComparePath::False
				}
			},
			Some(_) => ComparePath::False,
			None => if path == name { ComparePath::True } else { ComparePath::False },
		}
	}
}
#[test]
fn test_compare_id() {
	assert_eq!(ComparePath::cmp("foo", "foo"), ComparePath::True);
	assert_eq!(ComparePath::cmp("foo.bar", "foo"), ComparePath::Part("bar"));
	assert_eq!(ComparePath::cmp("foo.bar", "foo.bar"), ComparePath::True);
	assert_eq!(ComparePath::cmp("foo.bar.baz", "foo.bar"), ComparePath::Part("baz"));
	assert_eq!(ComparePath::cmp("fooz", "foo"), ComparePath::False);
	assert_eq!(ComparePath::cmp("foo.bar.baz", "bar.baz"), ComparePath::False);
	assert_eq!(ComparePath::cmp("foo", "foo.bar"), ComparePath::False);
	// Degenerate cases: empty string node names
	assert_eq!(ComparePath::cmp("foo.", "foo"), ComparePath::Part(""));
	assert_eq!(ComparePath::cmp("foo.", "foo."), ComparePath::True);
	assert_eq!(ComparePath::cmp("foo", "foo."), ComparePath::False);
	assert_eq!(ComparePath::cmp("", ""), ComparePath::True);
	assert_eq!(ComparePath::cmp(".", ""), ComparePath::Part(""));
	assert_eq!(ComparePath::cmp(".", "."), ComparePath::True);
	assert_eq!(ComparePath::cmp("", "."), ComparePath::False);
	assert_eq!(ComparePath::cmp(".foo", "foo"), ComparePath::False);
	assert_eq!(ComparePath::cmp(".foo", ".foo"), ComparePath::True);
	assert_eq!(ComparePath::cmp("foo", ".foo"), ComparePath::False);
}

//----------------------------------------------------------------

/// Finds a cvar by its path and invokes the closure with it.
///
/// If there are multiple nodes with the same name, the closure is called for every match.
/// This allows interesting features where eg. the same path is both an action and a property.
///
/// Returns false if no nodes were found with this path, the closure has not been called.
#[inline]
pub fn find<F: FnMut(&mut dyn INode)>(root: &mut dyn IVisit, path: &str, mut f: F) -> bool {
	find_rec(root, path, &mut f)
}
#[inline]
fn find_rec(list: &mut dyn IVisit, path: &str, f: &mut dyn FnMut(&mut dyn INode)) -> bool {
	let mut found = false;
	list.visit(&mut |node| {
		match ComparePath::cmp(path, node.name()) {
			ComparePath::True => {
				f(node);
				found = true;
			},
			ComparePath::Part(tail) => {
				if let Node::List(list) = node.as_node() {
					found |= find_rec(list.as_ivisit(), tail, f);
				}
			},
			ComparePath::False => {},
		};
	});
	found
}

/// Walks all nodes in the cvar hierarchy and call the closure with the node along its full path.
#[inline]
pub fn walk<F: FnMut(&str, &mut dyn INode)>(root: &mut dyn IVisit, mut f: F) {
	let mut path = String::new();
	walk_rec(root, &mut path, &mut f);
}
#[inline]
fn walk_rec(list: &mut dyn IVisit, path: &mut String, f: &mut dyn FnMut(&str, &mut dyn INode)) {
	list.visit(&mut |node| {
		// Construct the path
		let len = path.len();
		if len > 0 {
			path.push('.');
		}
		path.push_str(node.name());
		// Tell our caller about the node
		f(&path, node);
		// Recursively visit list nodes
		if let Node::List(list) = node.as_node() {
			walk_rec(list.as_ivisit(), path, f);
		}
		// Pop off the name so we can reuse this string
		path.truncate(len);
	});
}

//----------------------------------------------------------------

/// Invokes an action.
///
/// Returns false if no action node was found at the given path.
#[inline]
pub fn invoke(root: &mut dyn IVisit, path: &str, args: &str, writer: &mut dyn IWrite) -> bool {
	let mut found = false;
	find(root, path, |node| {
		if let Node::Action(act) = node.as_node() {
			found = true;
			act.invoke(args, writer);
		}
	});
	found
}
