use super::*;

/// Console interface for actions to writer output to.
pub trait IWrite: any::Any + fmt::Write {}

impl dyn IWrite {
	/// Returns `true` if the inner type is the same as `T`.
	#[inline]
	pub fn is<T: any::Any>(&self) -> bool {
		(self as &dyn any::Any).is::<T>()
	}

	/// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
	#[inline]
	pub fn downcast_ref<T: any::Any>(&self) -> Option<&T> {
		(self as &dyn any::Any).downcast_ref::<T>()
	}

	/// Returns some mutable reference to the inner value if it is of type `T`, or `None` if it isn't.
	#[inline]
	pub fn downcast_mut<T: any::Any>(&mut self) -> Option<&mut T> {
		(self as &mut dyn any::Any).downcast_mut::<T>()
	}
}

impl IWrite for String {}

/// Null writer.
///
/// Helper which acts as `dev/null`, any writes disappear in the void.
pub struct NullWriter;

impl fmt::Write for NullWriter {
	fn write_str(&mut self, _s: &str) -> fmt::Result { Ok(()) }
	fn write_char(&mut self, _c: char) -> fmt::Result { Ok(()) }
	fn write_fmt(&mut self, _args: fmt::Arguments) -> fmt::Result { Ok(()) }
}

impl IWrite for NullWriter {}

/// Io writer.
///
/// Helper which adapts to any `std::io::Write` objects such as stdout.
#[cfg(feature = "std")]
pub struct IoWriter<W>(pub W);

#[cfg(feature = "std")]
impl<W: io::Write> fmt::Write for IoWriter<W> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		let Self(this) = self;
		io::Write::write_all(this, s.as_bytes()).map_err(|_| fmt::Error)
	}
	fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
		let Self(this) = self;
		io::Write::write_fmt(this, args).map_err(|_| fmt::Error)
	}
}

#[cfg(feature = "std")]
impl<W: io::Write + 'static> IWrite for IoWriter<W> {}

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
