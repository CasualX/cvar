use super::*;

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
}

impl<'a, F: FnMut(&str, &mut dyn IWrite)> IAction for Action<'a, F> {
	fn invoke(&mut self, args: &str, writer: &mut dyn IWrite) {
		(self.invoke)(args, writer)
	}
}
