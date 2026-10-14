use super::*;

/// Context interface for actions to write output to and request data from.
pub trait IContext: fmt::Write {
	/// Returns a shared reference to context data of the requested type.
	fn request_any_ref(&self, _ty: any::TypeId) -> Option<&dyn any::Any> { None }

	/// Returns a mutable reference to context data of the requested type.
	fn request_any_mut(&mut self, _ty: any::TypeId) -> Option<&mut dyn any::Any> { None }
}

impl dyn IContext + '_ {
	/// Requests shared context data of type `T` through [`IContext::request_any_ref`] or `None`.
	#[inline]
	pub fn request_ref<T: any::Any>(&self) -> Option<&T> {
		IContext::request_any_ref(self, any::TypeId::of::<T>()).and_then(|v| v.downcast_ref::<T>())
	}

	/// Requests mutable context data of type `T` through [`IContext::request_any_mut`] or `None`.
	#[inline]
	pub fn request_mut<T: any::Any>(&mut self) -> Option<&mut T> {
		IContext::request_any_mut(self, any::TypeId::of::<T>()).and_then(|v| v.downcast_mut::<T>())
	}

	/// Writes `error: `, the display message, and a trailing newline.
	#[inline]
	pub fn write_error<T: ?Sized + fmt::Display>(&mut self, err: &T) -> fmt::Result {
		writeln!(self, "error: {err}")
	}

	#[cfg(feature = "type_name")]
	#[inline]
	pub(crate) fn write_mismatched_types<T: IValue>(&mut self, val: &dyn IValue) -> fmt::Result {
		self.write_error(&format_args!("mismatched types: expected `{}`, found `{}`", any::type_name::<T>(), val.type_name()))
	}

	#[cfg(not(feature = "type_name"))]
	#[inline]
	pub(crate) fn write_mismatched_types<T: IValue>(&mut self, _val: &dyn IValue) -> fmt::Result {
		self.write_error("mismatched types")
	}
}

impl IContext for String {}

/// Null context.
///
/// Helper which acts as `dev/null`, any writes disappear in the void.
pub struct NullContext;

impl fmt::Write for NullContext {
	fn write_str(&mut self, _s: &str) -> fmt::Result { Ok(()) }
	fn write_char(&mut self, _c: char) -> fmt::Result { Ok(()) }
	fn write_fmt(&mut self, _args: fmt::Arguments) -> fmt::Result { Ok(()) }
}

impl IContext for NullContext {}

/// Io context.
///
/// Helper which adapts to any `std::io::Write` objects such as stdout.
#[cfg(feature = "std")]
pub struct IoWriter<W>(pub W);

#[cfg(feature = "std")]
impl<W: io::Write> fmt::Write for IoWriter<W> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		let IoWriter(this) = self;
		io::Write::write_all(this, s.as_bytes()).map_err(|_| fmt::Error)
	}
	fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
		let IoWriter(this) = self;
		io::Write::write_fmt(this, args).map_err(|_| fmt::Error)
	}
}

#[cfg(feature = "std")]
impl<W: io::Write> IContext for IoWriter<W> {}

#[cfg(feature = "std")]
impl IoWriter<io::Stdout> {
	#[inline]
	pub fn stdout() -> IoWriter<io::Stdout> {
		IoWriter(io::stdout())
	}
}

#[cfg(feature = "std")]
impl IoWriter<io::Stderr> {
	#[inline]
	pub fn stderr() -> IoWriter<io::Stderr> {
		IoWriter(io::stderr())
	}
}
