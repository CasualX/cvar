use super::*;

//----------------------------------------------------------------

/// An action or property operation failed.
///
/// Specific error details are reported through [`IContext`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ActionError;

/// Result of an action or property operation.
pub type ActionResult = Result<(), ActionError>;

impl From<fmt::Error> for ActionError {
	fn from(_: fmt::Error) -> Self {
		ActionError
	}
}

impl fmt::Display for ActionError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("action failed")
	}
}

impl error::Error for ActionError {}

//----------------------------------------------------------------

/// Describes how an action participates in configuration operations.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct ActionMetadata {
	/// Invoke this action during resets, in visitor order, with empty arguments and a [`NullContext`].
	pub change_callback: bool,
}

impl ActionMetadata {
	pub const DEFAULT: ActionMetadata = ActionMetadata {
		change_callback: false,
	};
}

/// Action node interface.
///
/// Provides an object safe interface for actions, type erasing its implementation.
pub trait IAction: INode {
	/// Returns the action's metadata.
	fn metadata(&self) -> &ActionMetadata;

	/// Invokes the closure associated with the Action.
	///
	/// Given an argument string and a context for output and application data.
	///
	/// Returns `Ok(())` on success or `Err(ActionError)` on failure.
	/// Output, error details, and application data are passed through the context.
	fn invoke(&mut self, args: &str, context: &mut dyn IContext) -> ActionResult;
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
pub struct Action<'a, F: FnMut(&str, &mut dyn IContext) -> ActionResult> {
	name: &'a str,
	invoke: F,
	metadata: ActionMetadata,
}

/// Creates a new Action instance.
#[allow(non_snake_case)]
#[inline]
pub const fn Action<'a, F: FnMut(&str, &mut dyn IContext) -> ActionResult>(name: &'a str, invoke: F) -> Action<'a, F> {
	Action::new(name, invoke)
}

/// Creates a new OnChange instance.
#[allow(non_snake_case)]
#[inline]
pub const fn OnChange<'a, F: FnMut(&str, &mut dyn IContext) -> ActionResult>(name: &'a str, invoke: F) -> Action<'a, F> {
	Action::new(name, invoke).with_metadata(ActionMetadata {
		change_callback: true,
		..ActionMetadata::DEFAULT
	})
}

impl<'a, F: FnMut(&str, &mut dyn IContext) -> ActionResult> Action<'a, F> {
	/// Creates a new instance.
	#[inline]
	pub const fn new(name: &'a str, invoke: F) -> Action<'a, F> {
		Action { name, invoke, metadata: ActionMetadata::DEFAULT }
	}

	/// Sets the action's metadata.
	#[inline]
	pub const fn with_metadata(mut self, metadata: ActionMetadata) -> Self {
		self.metadata = metadata;
		self
	}
}

impl<'a, F: FnMut(&str, &mut dyn IContext) -> ActionResult> INode for Action<'a, F> {
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Action(self)
	}
}

impl<'a, F: FnMut(&str, &mut dyn IContext) -> ActionResult> IAction for Action<'a, F> {
	fn metadata(&self) -> &ActionMetadata {
		&self.metadata
	}

	fn invoke(&mut self, args: &str, context: &mut dyn IContext) -> ActionResult {
		(self.invoke)(args, context)
	}
}
