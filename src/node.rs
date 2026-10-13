use super::*;

/// Node interface.
pub trait INode {
	/// Returns the node name.
	fn name(&self) -> &str;

	/// Downcasts to a more specific node interface.
	fn as_node(&mut self) -> Node<'_>;
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
}

/// Enumerates derived interfaces for downcasting.
#[derive(Debug)]
pub enum Node<'a> {
	Prop(&'a mut dyn IProperty),
	List(&'a mut dyn IList),
	Action(&'a mut dyn IAction),
}
