use super::*;

/// Context interface for actions to write output to and request data from.
pub trait IContext {
	/// Writes a string, ignoring output errors.
	fn write_str(&mut self, s: &str);

	/// Writes a character, ignoring output errors.
	fn write_char(&mut self, c: char) {
		self.write_str(c.encode_utf8(&mut [0; 4]));
	}

	/// Writes formatted arguments, ignoring output errors.
	fn write_fmt(&mut self, args: fmt::Arguments<'_>) {
		struct Writer<'a, T: ?Sized>(&'a mut T);
		impl<T: IContext + ?Sized> fmt::Write for Writer<'_, T> {
			fn write_str(&mut self, s: &str) -> fmt::Result {
				self.0.write_str(s);
				Ok(())
			}
			fn write_char(&mut self, c: char) -> fmt::Result {
				self.0.write_char(c);
				Ok(())
			}
		}
		_ = fmt::write(&mut Writer(self), args);
	}

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
	pub fn write_error<T: ?Sized + fmt::Display>(&mut self, err: &T) {
		writeln!(self, "error: {err}")
	}

	#[cfg(feature = "type_name")]
	#[inline]
	pub(crate) fn write_mismatched_types<T: IValue>(&mut self, val: &dyn IValue) {
		self.write_error(&format_args!("mismatched types: expected `{}`, found `{}`", any::type_name::<T>(), val.type_name()))
	}

	#[cfg(not(feature = "type_name"))]
	#[inline]
	pub(crate) fn write_mismatched_types<T: IValue>(&mut self, _val: &dyn IValue) {
		self.write_error("mismatched types")
	}
}

impl IContext for String {
	fn write_str(&mut self, s: &str) { self.push_str(s); }
	fn write_char(&mut self, c: char) { self.push(c); }
}

/// Null context.
///
/// Helper which acts as `dev/null`, any writes disappear in the void.
pub struct NullContext;

impl IContext for NullContext {
	fn write_str(&mut self, _s: &str) {}
	fn write_char(&mut self, _c: char) {}
	fn write_fmt(&mut self, _args: fmt::Arguments<'_>) {}
}

/// Io context.
///
/// Helper which adapts to any `std::io::Write` objects such as stdout.
/// Output errors are ignored.
#[cfg(feature = "std")]
pub struct IoWriter<W>(pub W);

#[cfg(feature = "std")]
impl<W: io::Write> IContext for IoWriter<W> {
	fn write_str(&mut self, s: &str) {
		let IoWriter(this) = self;
		_ = io::Write::write_all(this, s.as_bytes());
	}
	fn write_fmt(&mut self, args: fmt::Arguments<'_>) {
		let IoWriter(this) = self;
		_ = io::Write::write_fmt(this, args);
	}
}

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
