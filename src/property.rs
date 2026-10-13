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
pub struct Property<'a, T: 'static> {
	name: &'a str,
	variable: &'a mut T,
	default: &'a T,
}

#[allow(non_snake_case)]
#[inline]
pub fn Property<'a, T>(name: &'a str, variable: &'a mut T, default: &'a T) -> Property<'a, T> {
	Property { name, variable, default }
}

impl<'a, T> Property<'a, T> {
	#[inline]
	pub fn new(name: &'a str, variable: &'a mut T, default: &'a T) -> Property<'a, T> {
		Property { name, variable, default }
	}
}

impl<'a, T> INode for Property<'a, T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: error::Error + Send + Sync + 'static
{
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}
}

impl<'a, T> IProperty for Property<'a, T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: error::Error + Send + Sync + 'static
{
	fn get_value(&self) -> &dyn IValue {
		&*self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
		let Some(val) = val.downcast_ref::<T>() else {
			let _ = write_mismatched_types::<T>(writer, val);
			return false
		};
		self.variable.clone_from(val);
		true
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
		self.variable.clone_from(self.default);
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
/// Clamped assignments return `true`. Values incomparable with a bound (such as
/// `NaN`) and invalid bounds are rejected, leaving the variable unchanged.
pub struct ClampedProp<'a, T: 'static> {
	name: &'a str,
	variable: &'a mut T,
	default: &'a T,
	min: Option<&'a T>,
	max: Option<&'a T>,
}

#[allow(non_snake_case)]
#[inline]
pub fn ClampedProp<'a, T>(name: &'a str, variable: &'a mut T, default: &'a T, min: Option<&'a T>, max: Option<&'a T>) -> ClampedProp<'a, T> {
	ClampedProp { name, variable, default, min, max }
}

impl<'a, T> ClampedProp<'a, T> {
	#[inline]
	pub fn new(name: &'a str, variable: &'a mut T, default: &'a T, min: Option<&'a T>, max: Option<&'a T>) -> ClampedProp<'a, T> {
		ClampedProp { name, variable, default, min, max }
	}
}

impl<'a, T> INode for ClampedProp<'a, T>
	where T: Clone + Default + PartialEq + PartialOrd + fmt::Display + FromStr,
	      T::Err: error::Error + Send + Sync + 'static
{
	fn name(&self) -> &str {
		self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}
}

impl<'a, T> IProperty for ClampedProp<'a, T>
	where T: Clone + Default + PartialEq + PartialOrd + fmt::Display + FromStr,
	      T::Err: error::Error + Send + Sync + 'static
{
	fn get_value(&self) -> &dyn IValue {
		&*self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
		let Some(val) = val.downcast_ref::<T>() else {
			let _ = write_mismatched_types::<T>(writer, val);
			return false
		};
		let Some(val) = clamp_bounds_inclusive(val, self.min, self.max) else {
			let _ = writer.write_str("cannot clamp value to bounds");
			return false
		};
		self.variable.clone_from(val);
		true
	}

	fn set(&mut self, val: &str, writer: &mut dyn IWrite) -> bool {
		match val.parse::<T>() {
			Ok(val) => {
				if let Some(val) = clamp_bounds_inclusive(&val, self.min, self.max) {
					self.variable.clone_from(val);
					true
				}
				else {
					let _ = writer.write_str("cannot clamp value to bounds");
					false
				}
			},
			Err(err) => {
				let _ = write_error(writer, &err);
				false
			},
		}
	}

	fn reset(&mut self) {
		self.variable.clone_from(self.default);
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
}

impl<'a, T: PartialEq + IValue> IProperty for ReadOnlyProp<'a, T> {
	fn get_value(&self) -> &dyn IValue {
		self.variable
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

#[allow(non_snake_case)]
#[inline]
pub fn OwnedProp<T>(name: String, variable: T, default: T) -> OwnedProp<T> {
	OwnedProp { name, variable, default }
}

impl<T> OwnedProp<T> {
	#[inline]
	pub fn new(name: String, variable: T, default: T) -> OwnedProp<T> {
		OwnedProp { name, variable, default }
	}
}

impl<T> INode for OwnedProp<T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: error::Error + Send + Sync + 'static
{
	fn name(&self) -> &str {
		&self.name
	}

	fn as_node(&mut self) -> Node<'_> {
		Node::Prop(self)
	}
}

impl<T> IProperty for OwnedProp<T>
	where T: Clone + Default + PartialEq + fmt::Display + FromStr,
	      T::Err: error::Error + Send + Sync + 'static
{
	fn get_value(&self) -> &dyn IValue {
		&self.variable
	}

	fn set_value(&mut self, val: &dyn IValue, writer: &mut dyn IWrite) -> bool {
		let Some(val) = val.downcast_ref::<T>() else {
			let _ = write_mismatched_types::<T>(writer, val);
			return false
		};
		self.variable.clone_from(val);
		true
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
