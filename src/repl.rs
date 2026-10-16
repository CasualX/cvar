//! Interact with the configuration variables.
//!
//! The design of this library makes recursive depth-first pre-order traversal the only feasible method to walk the cvars.
//!
//! This trade-off allows the hierarchy to be constructed lazily with very convenient stack-allocated resources.

use super::*;

pub mod parse;

/// Pokes the cvar tree.
///
/// This function combines the behavior of [`print()`], [`get`], [`set`], and [`invoke`].
///
/// Returns `Ok(true)` if the path or any dotted descendants matched and all operations succeeded,
/// `Ok(false)` if nothing matched, or `Err(ActionError)` if a property update or action failed.
///
/// Error details are written to the context. All matching nodes are visited, even after a failure.
///
/// Change callbacks are only invoked when `args` is `Some`, ordinary actions are always invoked.
/// See [`OnChange`] for the notification contract.
///
/// Property arguments are decoded with [`parse::string`],
/// allowing empty strings, surrounding whitespace, and escaped characters.
/// Actions, including change callbacks, receive the original arguments.
/// Use [`set`] or [`set_value`] to assign values verbatim.
///
/// Nodes below the path's dotted prefix are listed, including nodes with dotted
/// names and descendants of real lists.
pub fn poke(root: &mut dyn IVisit, path: &str, args: Option<&str>, context: &mut dyn IContext) -> Result<bool, ActionError> {
	let mut found = false;
	let mut result = Ok(());
	if !path.is_empty() {
		walk(root, |node_path, node| {
			match ComparePath::cmp(node_path, path) {
				ComparePath::Descendant => {
					_print_node(node, node_path, context);
					found = true;
					return true; // Visit list children below the requested path.
				},
				ComparePath::Ancestor => return true, // Descend toward the requested path.
				ComparePath::Unrelated => return false,
				ComparePath::Equal => {},
			}
			found = true;
			match node.as_node() {
				Node::Prop(prop) => {
					if let Some(val) = args {
						let status = match parse::string(val) {
							Ok(val) => prop.set(&val, context),
							Err(err) => {
								context.write_error(&err);
								Err(ActionError)
							},
						};
						result = result.and(status);
						if status.is_ok() {
							let value = prop.display_value().to_string();
							// cvar.prop "true"
							writeln!(context, "{path} {value:?}");
						}
					}
					else {
						let value = prop.display_value().to_string();
						// cvar.prop "true"
						writeln!(context, "{path} {value:?}");
					}
				},
				Node::List(_) => {}, // `walk` visits list children after this callback returns `true`.
				Node::Action(act) => {
					if args.is_some() || !act.metadata().change_callback {
						result = result.and(act.invoke(args.unwrap_or(""), context));
					}
				},
			}
			true
		});
	}
	else {
		_print_nodes(root, None, context);
		found = true;
	}
	result.map(|()| found)
}

fn _print_node(node: &mut dyn INode, path: &str, context: &mut dyn IContext) {
	match node.as_node() {
		Node::Prop(prop) => {
			let value = prop.display_value().to_string();
			write!(context, "{path} {value:?}");
		},
		Node::List(_) => {
			context.write_str(path);
			context.write_str("…");
		},
		Node::Action(_) => {
			context.write_str(path);
		},
	}
	context.write_str("\n");
}
fn _print_nodes(root: &mut dyn IVisit, path: Option<&str>, context: &mut dyn IContext) {
	root.visit(&mut |node| {
		let full_path = match path {
			Some(path) => format!("{path}.{}", node.name()),
			None => node.name().to_owned(),
		};
		_print_node(node, &full_path, context);
	});
}

//----------------------------------------------------------------

/// Sets a property's value parsed from a string.
///
/// If the path is an action it is invoked with the value as the argument.
///
/// Returns `Ok(true)` if at least one property or action matched and all operations succeeded,
/// `Ok(false)` if none matched, or `Err(ActionError)` if a property update or action failed.
///
/// Error details are written to the context. All matching nodes are visited, even after a failure.
#[inline]
pub fn set(root: &mut dyn IVisit, path: &str, val: &str, context: &mut dyn IContext) -> Result<bool, ActionError> {
	let mut found = false;
	let mut result = Ok(());
	find(root, path, |node| {
		match node.as_node() {
			Node::Prop(prop) => {
				found = true;
				result = result.and(prop.set(val, context));
			},
			Node::List(_) => {},
			Node::Action(act) => {
				found = true;
				result = result.and(act.invoke(val, context));
			},
		}
	});
	result.map(|()| found)
}

/// Gets a property's value as a string.
///
/// Returns `None` if the path does not lead to a property.
#[inline]
pub fn get(root: &mut dyn IVisit, path: &str) -> Option<String> {
	let mut result = None;
	find(root, path, |node| {
		if let Node::Prop(prop) = node.as_node() {
			result = Some(prop.display_value().to_string());
		}
	});
	result
}

/// Sets a property's value directly.
///
/// If the path is an action it is invoked with the value as the argument.
///
/// Returns `Ok(true)` if at least one property or action matched and all operations succeeded,
/// `Ok(false)` if none matched, or `Err(ActionError)` if a property update or action failed.
///
/// Error details are written to the context. All matching nodes are visited, even after a failure.
#[inline]
pub fn set_value(root: &mut dyn IVisit, path: &str, val: &dyn IValue, context: &mut dyn IContext) -> Result<bool, ActionError> {
	let mut found = false;
	let mut result = Ok(());
	find(root, path, |node| {
		match node.as_node() {
			Node::Prop(prop) => {
				found = true;
				result = result.and(prop.set_value(val, context));
			},
			Node::List(_) => {},
			Node::Action(act) => {
				found = true;
				result = result.and(act.invoke(&val.to_string(), context));
			},
		}
	});
	result.map(|()| found)
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
/// Resets the exact path and all dotted descendants.
/// Returns `Ok(true)` if the path or any descendants matched and all operations succeeded,
/// `Ok(false)` if nothing matched, or `Err(ActionError)` if a property reset or change callback failed.
///
/// Matching actions marked as change callbacks are invoked in visitor order with empty arguments
/// and the supplied context. Other actions are skipped.
/// Error details are written to the context. All matching nodes are visited, even after a failure.
#[inline]
pub fn reset(root: &mut dyn IVisit, path: &str, context: &mut dyn IContext) -> Result<bool, ActionError> {
	let mut found = false;
	let mut result = Ok(());
	walk(root, |node_path, node| {
		match ComparePath::cmp(node_path, path) {
			ComparePath::Ancestor => return true,
			ComparePath::Unrelated => return false,
			ComparePath::Equal | ComparePath::Descendant => {},
		}
		found = true;
		match node.as_node() {
			Node::Prop(prop) => result = result.and(prop.reset(context)),
			Node::List(_) => {}, // `walk` visits list children after this callback returns `true`.
			Node::Action(act) => {
				if act.metadata().change_callback {
					result = result.and(act.invoke("", context));
				}
			},
		}
		true
	});
	result.map(|()| found)
}

/// Resets all properties to their default.
///
/// Actions marked as change callbacks are invoked in visitor order with empty arguments
/// and the supplied context. Other actions are skipped.
///
/// Returns `Err(ActionError)` if any property reset or change callback failed.
/// Error details are written to the context. All nodes are visited, even after a failure.
#[inline]
pub fn reset_all(root: &mut dyn IVisit, context: &mut dyn IContext) -> ActionResult {
	let mut result = Ok(());
	walk(root, |_, node| {
		match node.as_node() {
			Node::Prop(prop) => result = result.and(prop.reset(context)),
			Node::List(_) => {}, // `walk` visits list children after this callback returns `true`.
			Node::Action(act) => {
				if act.metadata().change_callback {
					result = result.and(act.invoke("", context));
				}
			},
		}
		true
	});
	result
}

/// Lists the exact path and all dotted descendants.
///
/// An empty path lists the visitor's immediate children.
#[inline]
pub fn print(root: &mut dyn IVisit, path: &str, context: &mut dyn IContext) {
	if !path.is_empty() {
		let mut found = false;
		walk(root, |node_path, node| {
			match ComparePath::cmp(node_path, path) {
				ComparePath::Ancestor => return true,
				ComparePath::Unrelated => return false,
				ComparePath::Equal | ComparePath::Descendant => {},
			}
			found = true;
			_print_node(node, node_path, context);
			true // `walk` visits list children after this callback returns `true`.
		});
		if !found {
			context.write_error(&format_args!("path {path:?} does not exist"));
		}
	}
	else {
		_print_nodes(root, None, context);
	}
}

//----------------------------------------------------------------

/// The first path's relationship to the second, using `.` as the separator.
///
/// Node names are allowed to contain `.`, eg: `Action::new("foo.bar", |args, context| Ok(()))`.
/// This does not create a `foo` list node, but merely allows the node to pretend to be part of one.
///
/// Node names are allowed to be the empty string, while confusing there's nothing special about it.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ComparePath {
	Unrelated,
	Equal,
	Ancestor,
	Descendant,
}
impl ComparePath {
	#[inline]
	fn cmp(path: &str, other: &str) -> ComparePath {
		let (shorter, longer, relation) = match path.len().cmp(&other.len()) {
			core::cmp::Ordering::Less => (path, other, ComparePath::Ancestor),
			core::cmp::Ordering::Greater => (other, path, ComparePath::Descendant),
			core::cmp::Ordering::Equal => return if path == other { ComparePath::Equal } else { ComparePath::Unrelated },
		};
		if longer.strip_prefix(shorter).is_some_and(|suffix| suffix.starts_with('.')) {
			relation
		}
		else {
			ComparePath::Unrelated
		}
	}
}
#[test]
fn test_compare_path() {
	use ComparePath::{Ancestor, Descendant, Equal, Unrelated};
	for (path, other, expected) in [
		("foo", "foo", Equal),
		("foo.bar", "foo", Descendant),
		("foo.bar", "foo.bar", Equal),
		("foo.bar.baz", "foo.bar", Descendant),
		("fooz", "foo", Unrelated),
		("foo.bar.baz", "bar.baz", Unrelated),
		// Empty names still require a dot separator.
		("foo.", "foo", Descendant),
		("foo.", "foo.", Equal),
		("", "", Equal),
		(".", "", Descendant),
		(".", ".", Equal),
		(".foo", "foo", Unrelated),
		(".foo", ".foo", Equal),
		("foo", "", Unrelated),
		("é.子", "é", Descendant),
		("é", "e", Unrelated),
		("é", "éx", Unrelated),
	] {
		assert_eq!(ComparePath::cmp(path, other), expected, "{path:?}, {other:?}");
		let reversed = match expected {
			Ancestor => Descendant,
			Descendant => Ancestor,
			Equal => Equal,
			Unrelated => Unrelated,
		};
		assert_eq!(ComparePath::cmp(other, path), reversed, "{other:?}, {path:?}");
	}
}

//----------------------------------------------------------------

/// Finds nodes at the exact path and invokes the closure with each match.
///
/// If there are multiple nodes with the same name, the closure is called for every match.
/// This allows interesting features where eg. the same path is both an action and a property.
///
/// Returns false if no nodes were found with this path, the closure has not been called.
#[inline]
pub fn find<F: FnMut(&mut dyn INode)>(root: &mut dyn IVisit, path: &str, mut f: F) -> bool {
	let mut found = false;
	walk(root, |node_path, node| {
		match ComparePath::cmp(node_path, path) {
			ComparePath::Equal => {
				found = true;
				f(node);
				false
			},
			ComparePath::Ancestor => true,
			ComparePath::Descendant | ComparePath::Unrelated => false,
		}
	});
	found
}

/// Walks nodes in the cvar hierarchy and calls the closure with each node and its full path.
///
/// Return `true` to visit a list's children, or `false` to skip them.
/// Sibling nodes are always visited. The return value has no effect on properties or actions.
#[inline]
pub fn walk<F: FnMut(&str, &mut dyn INode) -> bool>(root: &mut dyn IVisit, mut f: F) {
	let mut path = String::new();
	walk_rec(root, &mut path, &mut f);
}
#[inline]
fn walk_rec(list: &mut dyn IVisit, path: &mut String, f: &mut dyn FnMut(&str, &mut dyn INode) -> bool) {
	list.visit(&mut |node| {
		// Construct the path
		let len = path.len();
		path.push_str(node.name());
		// Tell our caller about the node
		if f(path, node) {
			// Recursively visit list nodes when requested
			if let Node::List(list) = node.as_node() {
				path.push('.');
				walk_rec(list.visitor(), path, f);
			}
		}
		// Pop off the name so we can reuse this string
		path.truncate(len);
	});
}

//----------------------------------------------------------------

/// Invokes an action.
///
/// Returns `Ok(true)` if at least one action matched and all matching actions succeeded,
/// `Ok(false)` if no action matched, or `Err(ActionError)` if any matching action failed.
///
/// Actions report error details through the context. All matching actions are invoked, even after a failure.
#[inline]
pub fn invoke(root: &mut dyn IVisit, path: &str, args: &str, context: &mut dyn IContext) -> Result<bool, ActionError> {
	let mut found = false;
	let mut result = Ok(());
	find(root, path, |node| {
		if let Node::Action(act) = node.as_node() {
			found = true;
			result = result.and(act.invoke(args, context));
		}
	});
	result.map(|()| found)
}
