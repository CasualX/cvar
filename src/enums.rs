use super::*;

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
/// Implements `Display` and `FromStr` for the enum, allowing it to be used as a cvar property.
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
/// 		f(&mut cvar::Property("mode", &mut self.mode, &TestEnum::First));
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
/// let mut writer = cvar::NullWriter;
/// assert!(cvar::repl::set(&mut config, "mode", "Third", &mut writer));
/// assert_eq!(config.mode, TestEnum::Third);
/// ```
#[macro_export]
macro_rules! enum_string {
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
