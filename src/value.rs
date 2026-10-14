use super::*;

//----------------------------------------------------------------

/// Property values that can be displayed and downcast to their concrete type.
///
/// Implemented automatically for all `'static` types that implement [`fmt::Display`].
/// Reading a value does not require cloning, comparison, or parsing.
/// Property implementations determine which operations are supported on the value.
pub trait IValue: any::Any + fmt::Display {
	/// Returns the name of the concrete type.
	#[cfg(feature = "type_name")]
	fn type_name(&self) -> &str {
		any::type_name::<Self>()
	}
}
impl fmt::Debug for dyn IValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, f)
	}
}

impl dyn IValue {
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
}

impl<T: any::Any + fmt::Display> IValue for T {}

//----------------------------------------------------------------

#[allow(non_snake_case)]
#[inline]
pub fn HexProp<'a, T>(name: &'a str, value: &'a mut T, default: &'a T) -> Property<'a, HexValue<T>> {
	Property(name, HexValue::from_mut(value), HexValue::from_ref(default))
}

/// Format the value as hexadecimal.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct HexValue<T>(pub T);

impl<T> HexValue<T> {
	/// Transmutes to a `&HexValue<T>`.
	#[inline]
	pub fn from_ref(value: &T) -> &Self {
		// SAFETY: HexValue is repr(transparent) over T.
		unsafe { &*(value as *const T as *const Self) }
	}
	/// Transmutes to a `&mut HexValue<T>`.
	#[inline]
	pub fn from_mut(value: &mut T) -> &mut Self {
		// SAFETY: HexValue is repr(transparent) over T, and the borrow is exclusive.
		unsafe { &mut *(value as *mut T as *mut Self) }
	}
}

impl<T> From<T> for HexValue<T> {
	#[inline]
	fn from(value: T) -> Self {
		Self(value)
	}
}
impl<T> AsRef<T> for HexValue<T> {
	#[inline]
	fn as_ref(&self) -> &T {
		&self.0
	}
}
impl<T> AsMut<T> for HexValue<T> {
	#[inline]
	fn as_mut(&mut self) -> &mut T {
		&mut self.0
	}
}

macro_rules! impl_HexValue {
	($ty:ty, $unsigned:ty) => {
		impl fmt::Display for HexValue<$ty> {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				if self.0 == 0 {
					f.write_str("0")
				}
				else {
					write!(f, "{:#x}", self.0)
				}
			}
		}

		impl FromStr for HexValue<$ty> {
			type Err = num::ParseIntError;
			fn from_str(mut s: &str) -> Result<Self, num::ParseIntError> {
				let mut not = false;
				if let Some(rest) = s.strip_prefix("!") {
					s = rest;
					not = true;
				}
				let decimal = s;

				let mut negate = false;
				if let Some(rest) = s.strip_prefix("-") {
					s = rest;
					negate = true;
				}

				let mut radix = 10;
				if let Some(rest) = s.strip_prefix("0x") {
					s = rest;
					radix = 16;
				}

				let mut value = if radix == 16 {
					// Parse the full bit pattern, including signed values' sign bit.
					let value = <$unsigned>::from_str_radix(s, radix)? as $ty;
					if negate { value.wrapping_neg() } else { value }
				}
				else if <$ty>::MIN == 0 {
					let value = <$ty>::from_str_radix(s, radix)?;
					if negate { value.wrapping_neg() } else { value }
				}
				else {
					// Keep the sign when parsing decimal so the minimum value is valid.
					<$ty>::from_str_radix(decimal, radix)?
				};

				if not {
					value = !value;
				}

				Ok(HexValue(value))
			}
		}
	};
}

impl_HexValue!(u64, u64);
impl_HexValue!(u32, u32);
impl_HexValue!(u16, u16);
impl_HexValue!(u8, u8);

impl_HexValue!(i64, u64);
impl_HexValue!(i32, u32);
impl_HexValue!(i16, u16);
impl_HexValue!(i8, u8);

#[test]
fn test_hex_values() {
	macro_rules! check_signed {
		($ty:ty) => {
			for value in [<$ty>::MIN, -1, 0, 1, <$ty>::MAX] {
				let hex = HexValue(value);
				assert_eq!(hex.to_string().parse::<HexValue<$ty>>(), Ok(hex));
				assert_eq!(value.to_string().parse::<HexValue<$ty>>(), Ok(hex));
			}
		};
	}
	check_signed!(i8);
	check_signed!(i16);
	check_signed!(i32);
	check_signed!(i64);
	assert_eq!("-0x80".parse::<HexValue<i8>>(), Ok(HexValue(i8::MIN)));
	assert_eq!("!0xff".parse::<HexValue<i8>>(), Ok(HexValue(0)));
	assert_eq!("!-128".parse::<HexValue<i8>>(), Ok(HexValue(127)));
	assert_eq!("-1".parse::<HexValue<u8>>(), Ok(HexValue(u8::MAX)));
	assert!("0x100".parse::<HexValue<u8>>().is_err());
	assert!("128".parse::<HexValue<i8>>().is_err());
	assert!("-129".parse::<HexValue<i8>>().is_err());
}
