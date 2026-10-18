use super::*;

/// Types with a fixed set of suggested strings in their ordinary parsing format.
///
/// Implemented for `bool` and enums declared with [`enum_string!`]. Custom enums
/// can implement this trait to use [`EnumProp`] without changing their parser.
pub trait EnumString {
	/// Suggested strings in declaration order.
	const VALUES: &'static [&'static str];
}

impl EnumString for bool {
	const VALUES: &'static [&'static str] = &["false", "true"];
}

/// A zero-sized provider for a type's [`EnumString::VALUES`].
///
/// Used automatically by [`EnumProp`] and [`BoolProp`]. Pass
/// `EnumValues::<T>::new()` to a property's `with_values` method to use the same
/// suggestions on [`ReadOnlyProp`] or [`OwnedProp`]. No value of `T` is stored.
pub struct EnumValues<T>(core::marker::PhantomData<fn() -> T>);

impl<T> EnumValues<T> {
	/// Creates the marker without storing a value or a reference.
	#[inline]
	pub const fn new() -> EnumValues<T> {
		EnumValues(core::marker::PhantomData)
	}
}

impl<T> Default for EnumValues<T> {
	fn default() -> EnumValues<T> {
		EnumValues::new()
	}
}

impl<T: EnumString> ValuesProvider for EnumValues<T> {
	fn values(&self) -> Option<&[&str]> {
		Some(T::VALUES)
	}
}

/// Creates an ordinary [`Property`] with the type's fixed value suggestions.
///
/// ```
/// use cvar::{EnumProp, IProperty};
/// cvar::enum_string! {
/// 	#[derive(Clone, PartialEq)]
/// 	enum Mode { First, Second }
/// }
/// let mut mode = Mode::First;
/// let prop = EnumProp("mode", &mut mode, &Mode::First);
/// assert_eq!(prop.values(), Some(&["First", "Second"][..]));
/// ```
///
/// Suggestions do not restrict assignments. With custom formatting, override the
/// suggestions using [`Property::with_values`] to match the format's parser.
#[allow(non_snake_case)]
#[inline]
pub const fn EnumProp<'a, T: EnumString + 'static>(name: &'a str, variable: &'a mut T, default: &'a T) -> Property<'a, T, DefaultFormat, EnumValues<T>> {
	Property::new_with_values(name, variable, default, EnumValues::new())
}

/// Creates an ordinary [`Property`] suggesting `false` and `true`.
#[allow(non_snake_case)]
#[inline]
pub const fn BoolProp<'a>(name: &'a str, variable: &'a mut bool, default: &'a bool) -> Property<'a, bool, DefaultFormat, EnumValues<bool>> {
	EnumProp(name, variable, default)
}

/// Error type for parsing enum variants from strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseEnumError {
	pub ident: &'static str,
}

impl fmt::Display for ParseEnumError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "invalid {} variant", self.ident)
	}
}

impl error::Error for ParseEnumError {}

/// Macro to create enums that can be converted to and from strings.
///
/// Implements `Display`, `FromStr`, and [`EnumString`] for the enum.
/// Use [`EnumProp`] to register the generated variant suggestions.
///
/// ```
/// cvar::enum_string! {
/// 	#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// 	pub enum TestEnum {
/// 		#[default]
/// 		First,
/// 		Second,
/// 		Third,
/// 	}
/// }
///
/// struct Config {
/// 	mode: TestEnum,
/// }
///
/// impl cvar::IVisit for Config {
/// 	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
/// 		f(&mut cvar::EnumProp("mode", &mut self.mode, &TestEnum::First));
/// 	}
/// }
///
/// assert_eq!(TestEnum::First.as_str(), "First");
/// assert_eq!("Second".parse::<TestEnum>(), Ok(TestEnum::Second));
/// assert!("Missing".parse::<TestEnum>().is_err());
///
/// let mut config = Config {
/// 	mode: TestEnum::First,
/// };
/// let mut context = cvar::NullContext;
/// assert_eq!(cvar::repl::set(&mut config, "mode", "Third", &mut context), Ok(true));
/// assert_eq!(config.mode, TestEnum::Third);
/// ```
#[macro_export]
macro_rules! enum_string {
	// Known limitation: cfg and conditional cfg_attr attributes inside this macro
	// are not propagated to generated impls or match arms, so disabling an enum or
	// variant can fail to compile. Attributes such as derive and default cannot
	// simply be copied to those locations; conditional attributes need filtering.
	(
		$(#[$meta:meta])*
		$vis:vis enum $name:ident {
			$(
				$(#[$variant_meta:meta])*
				$variant:ident $(= $value:expr)?
			),* $(,)?
		}
	) => {
		$(#[$meta])*
		$vis enum $name {
			$(
				$(#[$variant_meta])*
				$variant $(= $value)?
			),*
		}

		impl $name {
			/// Returns the string representation of the enum variant.
			#[inline]
			pub const fn as_str(&self) -> &'static str {
				match self {
					$(
						Self::$variant => ::core::stringify!($variant),
					)*
				}
			}
		}

		impl ::core::fmt::Display for $name {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				f.write_str(self.as_str())
			}
		}

		impl $crate::EnumString for $name {
			const VALUES: &'static [&'static str] = &[$(::core::stringify!($variant)),*];
		}

		impl ::core::str::FromStr for $name {
			type Err = $crate::ParseEnumError;

			fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
				match s {
					$(
						::core::stringify!($variant) => ::core::result::Result::Ok(Self::$variant),
					)*
					_ => ::core::result::Result::Err($crate::ParseEnumError {
						ident: ::core::stringify!($name),
					}),
				}
			}
		}
	};

	// Does not support generics
	(
		$(#[$meta:meta])*
		$vis:vis enum $name:ident < $($tt:tt)*
	) => {
		compile_error!("enum_string! does not support generic parameters");
	};
}
