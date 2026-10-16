use super::*;

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
	/// Gets a read-only view of the value.
	fn get_value(&self) -> &dyn IValue;

	/// Sets the value.
	///
	/// Returns `Err(ActionError)` on failure and writes error details to the context.
	fn set_value(&mut self, val: &dyn IValue, context: &mut dyn IContext) -> ActionResult;

	/// Sets the value parsed from string.
	///
	/// Returns `Err(ActionError)` on failure and writes error details to the context.
	fn set(&mut self, val: &str, context: &mut dyn IContext) -> ActionResult;

	/// Resets the value to its default.
	///
	/// Returns `Err(ActionError)` on failure and writes error details to the context.
	fn reset(&mut self, context: &mut dyn IContext) -> ActionResult;

	/// Gets a read-only view of the default value.
	fn default_value(&self) -> &dyn IValue;

	/// Returns the state of the property.
	fn state(&self) -> PropState;

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
		#[cfg(feature = "type_name")]
		debug.field("type", &self.type_name());
		debug.field("values", &self.values());
		debug.finish_non_exhaustive()
	}
}

//----------------------------------------------------------------

/// Property node.
pub struct Property<'a, T: 'static> {
	name: &'a str,
	variable: &'a mut T,
	default: &'a T,
}

/// Creates a new Property instance.
#[allow(non_snake_case)]
#[inline]
pub const fn Property<'a, T>(name: &'a str, variable: &'a mut T, default: &'a T) -> Property<'a, T> {
	Property { name, variable, default }
}

impl<'a, T> Property<'a, T> {
	/// Creates a new instance.
	#[inline]
	pub const fn new(name: &'a str, variable: &'a mut T, default: &'a T) -> Property<'a, T> {
		Property { name, variable, default }
	}
}

impl<'a, T> INode for Property<'a, T>
	where T: Clone + PartialEq + fmt::Display + FromStr,
	      T::Err: fmt::Display
{
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}
}

impl<'a, T> IProperty for Property<'a, T>
	where T: Clone + PartialEq + fmt::Display + FromStr,
	      T::Err: fmt::Display
{
	fn get_value(&self) -> &dyn IValue {
		&*self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, context: &mut dyn IContext) -> ActionResult {
		let Some(val) = val.downcast_ref::<T>() else {
			context.write_mismatched_types::<T>(val);
			return Err(ActionError)
		};
		self.variable.clone_from(val);
		Ok(())
	}

	fn set(&mut self, val: &str, context: &mut dyn IContext) -> ActionResult {
		match val.parse::<T>() {
			Ok(val) => {
				*self.variable = val;
				Ok(())
			},
			Err(err) => {
				context.write_error(&err);
				Err(ActionError)
			},
		}
	}

	fn reset(&mut self, _context: &mut dyn IContext) -> ActionResult {
		self.variable.clone_from(self.default);
		Ok(())
	}

	fn default_value(&self) -> &dyn IValue {
		self.default
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
fn clamp_bounds_inclusive<'a, T: PartialOrd>(mut val: &'a T, min: Option<&'a T>, max: Option<&'a T>) -> Option<&'a T> {
	if let Some(min) = min {
		if val < min {
			val = min;
		}
	}
	if let Some(max) = max {
		if val > max {
			val = max;
		}
	}
	if min.is_none_or(|min| val >= min) && max.is_none_or(|max| val <= max) {
		Some(val)
	}
	else {
		None
	}
}

/// Property node whose setters clamp values to inclusive bounds.
///
/// Values below the minimum or above the maximum are replaced with that bound.
/// Clamped assignments return `Ok(())`. Values incomparable with a bound (such as
/// `NaN`) and invalid bounds are rejected, leaving the variable unchanged.
///
/// The default value should be within the inclusive bounds.
pub struct ClampedProp<'a, T: 'static> {
	name: &'a str,
	variable: &'a mut T,
	default: &'a T,
	min: Option<&'a T>,
	max: Option<&'a T>,
}

/// Creates a new ClampedProp instance.
#[allow(non_snake_case)]
#[inline]
pub const fn ClampedProp<'a, T>(name: &'a str, variable: &'a mut T, default: &'a T, min: Option<&'a T>, max: Option<&'a T>) -> ClampedProp<'a, T> {
	ClampedProp { name, variable, default, min, max }
}

impl<'a, T> ClampedProp<'a, T> {
	/// Creates a new instance.
	#[inline]
	pub const fn new(name: &'a str, variable: &'a mut T, default: &'a T, min: Option<&'a T>, max: Option<&'a T>) -> ClampedProp<'a, T> {
		ClampedProp { name, variable, default, min, max }
	}
}

impl<'a, T> INode for ClampedProp<'a, T>
	where T: Clone + PartialEq + PartialOrd + fmt::Display + FromStr,
	      T::Err: fmt::Display
{
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}
}

impl<'a, T> IProperty for ClampedProp<'a, T>
	where T: Clone + PartialEq + PartialOrd + fmt::Display + FromStr,
	      T::Err: fmt::Display
{
	fn get_value(&self) -> &dyn IValue {
		&*self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, context: &mut dyn IContext) -> ActionResult {
		let Some(val) = val.downcast_ref::<T>() else {
			context.write_mismatched_types::<T>(val);
			return Err(ActionError)
		};
		let Some(val) = clamp_bounds_inclusive(val, self.min, self.max) else {
			context.write_error("cannot clamp value to bounds");
			return Err(ActionError)
		};
		self.variable.clone_from(val);
		Ok(())
	}

	fn set(&mut self, val: &str, context: &mut dyn IContext) -> ActionResult {
		match val.parse::<T>() {
			Ok(val) => {
				if let Some(val) = clamp_bounds_inclusive(&val, self.min, self.max) {
					self.variable.clone_from(val);
					Ok(())
				}
				else {
					context.write_error("cannot clamp value to bounds");
					Err(ActionError)
				}
			},
			Err(err) => {
				context.write_error(&err);
				Err(ActionError)
			},
		}
	}

	fn reset(&mut self, _context: &mut dyn IContext) -> ActionResult {
		// Default value not clamped against bounds...
		self.variable.clone_from(self.default);
		Ok(())
	}

	fn default_value(&self) -> &dyn IValue {
		self.default
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

/// Creates a new ReadOnlyProp instance.
#[allow(non_snake_case)]
#[inline]
pub const fn ReadOnlyProp<'a, T>(name: &'a str, variable: &'a T, default: &'a T) -> ReadOnlyProp<'a, T> {
	ReadOnlyProp { name, variable, default }
}

impl<'a, T> ReadOnlyProp<'a, T> {
	/// Creates a new instance.
	#[inline]
	pub const fn new(name: &'a str, variable: &'a T, default: &'a T) -> ReadOnlyProp<'a, T> {
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
}

impl<'a, T: PartialEq + IValue> IProperty for ReadOnlyProp<'a, T> {
	fn get_value(&self) -> &dyn IValue {
		self.variable
	}

	fn set_value(&mut self, _val: &dyn IValue, context: &mut dyn IContext) -> ActionResult {
		context.write_error("cannot set read-only property");
		Err(ActionError)
	}

	fn set(&mut self, _val: &str, context: &mut dyn IContext) -> ActionResult {
		context.write_error("cannot set read-only property");
		Err(ActionError)
	}

	fn reset(&mut self, context: &mut dyn IContext) -> ActionResult {
		context.write_error("cannot reset read-only property");
		Err(ActionError)
	}

	fn default_value(&self) -> &dyn IValue {
		self.default
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
#[non_exhaustive]
pub struct OwnedProp<T: 'static> {
	pub name: String,
	pub variable: T,
	pub default: T,
}

/// Creates a new OwnedProp instance.
#[allow(non_snake_case)]
#[inline]
pub const fn OwnedProp<T>(name: String, variable: T, default: T) -> OwnedProp<T> {
	OwnedProp { name, variable, default }
}

impl<T> OwnedProp<T> {
	/// Creates a new instance.
	#[inline]
	pub const fn new(name: String, variable: T, default: T) -> OwnedProp<T> {
		OwnedProp { name, variable, default }
	}
}

impl<T> INode for OwnedProp<T>
	where T: Clone + PartialEq + fmt::Display + FromStr,
	      T::Err: fmt::Display
{
	fn name(&self) -> &str {
		&self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}
}

impl<T> IProperty for OwnedProp<T>
	where T: Clone + PartialEq + fmt::Display + FromStr,
	      T::Err: fmt::Display
{
	fn get_value(&self) -> &dyn IValue {
		&self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, context: &mut dyn IContext) -> ActionResult {
		let Some(val) = val.downcast_ref::<T>() else {
			context.write_mismatched_types::<T>(val);
			return Err(ActionError)
		};
		self.variable.clone_from(val);
		Ok(())
	}

	fn set(&mut self, val: &str, context: &mut dyn IContext) -> ActionResult {
		match val.parse::<T>() {
			Ok(val) => {
				self.variable = val;
				Ok(())
			},
			Err(err) => {
				context.write_error(&err);
				Err(ActionError)
			},
		}
	}

	fn reset(&mut self, _context: &mut dyn IContext) -> ActionResult {
		self.variable.clone_from(&self.default);
		Ok(())
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
