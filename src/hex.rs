use super::*;

/// Hexadecimal property formatting and parsing.
///
/// Use with any property's `with_format` method to preserve the original value
/// type for typed reads and writes. Displays nonzero values with a `0x` prefix
/// and zero as `0`.
///
/// Accepts decimal input, `0x` hexadecimal input, and a leading `!` for bitwise
/// complement. Hexadecimal values use the integer's full bit pattern; negation
/// wraps for hexadecimal and unsigned decimal input.
///
/// ```
/// let mut bits = 42u32;
/// let mut root = cvar::Visit(|f| {
///     f(&mut cvar::Property("bits", &mut bits, &0).with_format(cvar::Hex));
/// });
/// assert_eq!(cvar::repl::get(&mut root, "bits").as_deref(), Some("0x2a"));
/// assert_eq!(cvar::repl::get_value::<u32>(&mut root, "bits"), Some(42));
/// cvar::repl::set(&mut root, "bits", "0xff", &mut cvar::NullContext).unwrap();
/// assert_eq!(cvar::repl::get_value::<u32>(&mut root, "bits"), Some(255));
/// ```
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Hex;

macro_rules! impl_Hex {
	($ty:ty, $unsigned:ty) => {
		impl ValueDisplay<$ty> for Hex {
			fn fmt(&self, value: &$ty, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				if *value == 0 {
					f.write_str("0")
				}
				else {
					write!(f, "{value:#x}")
				}
			}
		}

		impl ValueFromStr<$ty> for Hex {
			type Err = num::ParseIntError;
			fn parse(&self, mut s: &str) -> Result<$ty, num::ParseIntError> {
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

				Ok(value)
			}
		}
	};
}

impl_Hex!(u64, u64);
impl_Hex!(u32, u32);
impl_Hex!(u16, u16);
impl_Hex!(u8, u8);

impl_Hex!(i64, u64);
impl_Hex!(i32, u32);
impl_Hex!(i16, u16);
impl_Hex!(i8, u8);

/// Creates a property with hexadecimal formatting and parsing.
///
/// Equivalent to `Property(name, value, default).with_format(Hex)`.
/// Typed reads and writes use the original value type `T`.
#[allow(non_snake_case)]
#[inline]
pub fn HexProp<'a, T>(name: &'a str, value: &'a mut T, default: &'a T) -> Property<'a, T, Hex> {
	Property(name, value, default).with_format(Hex)
}

#[test]
fn hex_format_and_parse() {
	assert_eq!(ReadOnlyProp("value", &-1i8, &0).with_format(Hex).to_string(), "0xff");
	assert_eq!(ReadOnlyProp("value", &0i8, &0).with_format(Hex).to_string(), "0");
	for (input, expected) in [("-128", i8::MIN), ("-0x80", i8::MIN), ("0xff", -1), ("!0xff", 0)] {
		assert_eq!(ValueFromStr::<i8>::parse(&Hex, input), Ok(expected));
	}
	assert_eq!(ValueFromStr::<u8>::parse(&Hex, "-1"), Ok(u8::MAX));
	for input in ["0x100", "128", "-129"] {
		assert!(ValueFromStr::<i8>::parse(&Hex, input).is_err());
	}
}

#[test]
fn hex_properties_preserve_typed_access() {
	let mut borrowed = 42u32;
	let mut clamped = 42u32;
	let mut owned = OwnedProp(String::from("owned"), 42u32, 0).with_format(Hex);
	let mut root = Visit(|f| {
		f(&mut HexProp("borrowed", &mut borrowed, &0));
		f(&mut ClampedProp("clamped", &mut clamped, &0, None, Some(&100)).with_format(Hex));
		f(&mut ReadOnlyProp("read_only", &42u32, &0).with_format(Hex));
		f(&mut owned);
	});
	for path in ["borrowed", "clamped", "read_only", "owned"] {
		assert_eq!(repl::get(&mut root, path).as_deref(), Some("0x2a"));
		assert_eq!(repl::get_value::<u32>(&mut root, path), Some(42));
	}
	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "borrowed", Some("0xff"), &mut context), Ok(true));
	assert_eq!(context, "borrowed is \"0xff\"\n");
	assert_eq!(repl::get_value::<u32>(&mut root, "borrowed"), Some(255));
	context.clear();
	repl::print(&mut root, "borrowed", &mut context);
	assert_eq!(context, "borrowed is \"0xff\"\n");
	assert_eq!(repl::set_value(&mut root, "borrowed", &9u32, &mut NullContext), Ok(true));
	assert_eq!(repl::get(&mut root, "borrowed").as_deref(), Some("0x9"));
	assert_eq!(repl::set(&mut root, "clamped", "0xff", &mut NullContext), Ok(true));
	assert_eq!(repl::get_value::<u32>(&mut root, "clamped"), Some(100));
}
