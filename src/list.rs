use super::*;

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
/// let _ = cvar::repl::set(&mut visitor, "value", "42", &mut writer);
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
	fn visitor(&mut self) -> &mut dyn IVisit;
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
pub struct List<'a> {
	name: &'a str,
	visitor: &'a mut dyn IVisit,
}

#[allow(non_snake_case)]
#[inline]
pub fn List<'a>(name: &'a str, visitor: &'a mut dyn IVisit) -> List<'a> {
	List { name, visitor }
}

impl<'a> List<'a> {
	#[inline]
	pub fn new(name: &'a str, visitor: &'a mut dyn IVisit) -> List<'a> {
		List { name, visitor }
	}
}

impl<'a> INode for List<'a> {
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::List(self)
	}
}

impl<'a> IList for List<'a> {
	fn visitor(&mut self) -> &mut dyn IVisit {
		self.visitor
	}
}
