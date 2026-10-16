//! Simple command and argument parsing.

use alloc::borrow::Cow;
use alloc::string::String;
use core::{error, fmt};

/// A quoted string is incomplete or contains an invalid escape.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ParseStringError;

impl fmt::Display for ParseStringError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("invalid quoted string")
	}
}

impl error::Error for ParseStringError {}

/// Decodes a double-quoted string, or returns unquoted input unchanged.
///
/// Supports the escapes emitted by string [`Debug`](fmt::Debug) formatting:
/// `\0`, `\t`, `\n`, `\r`, `\\`, `\"`, and `\u{...}`. Also accepts `\'`
/// and ASCII `\xNN` escapes. Unicode escapes contain one to six hexadecimal
/// digits, optionally separated or followed by underscores.
///
/// Input starting with `"` must be exactly one complete quoted string, with no
/// trailing characters. Whitespace is preserved. Rust raw string syntax and
/// escaped line continuations are not interpreted.
///
/// The result borrows the input unless escapes need decoding.
///
/// ```
/// use cvar::repl::parse::string;
///
/// assert_eq!(string("Hello World").unwrap(), "Hello World");
/// assert_eq!(string(r#""""#).unwrap(), "");
/// assert_eq!(string(r#"" padded ""#).unwrap(), " padded ");
/// assert_eq!(string(r#""line\n\u{1f980}""#).unwrap(), "line\n🦀");
/// let original = "quotes: \" and backslash: \\ and nul: \0";
/// assert_eq!(string(&format!("{original:?}")).unwrap(), original);
/// assert!(string(r#""bad\q""#).is_err());
/// ```
pub fn string(input: &str) -> Result<Cow<'_, str>, ParseStringError> {
	let Some(inner) = input.strip_prefix('"') else {
		return Ok(Cow::Borrowed(input));
	};
	let inner = inner.strip_suffix('"').ok_or(ParseStringError)?;
	let Some(first_escape) = inner.find(['\\', '"']) else {
		return Ok(Cow::Borrowed(inner));
	};
	let mut output = String::with_capacity(inner.len());
	output.push_str(&inner[..first_escape]);
	let mut chars = inner[first_escape..].chars();
	while let Some(c) = chars.next() {
		match c {
			'"' => return Err(ParseStringError),
			'\\' => {
				let escaped = match chars.next().ok_or(ParseStringError)? {
					'0' => '\0',
					't' => '\t',
					'n' => '\n',
					'r' => '\r',
					'\\' => '\\',
					'"' => '"',
					'\'' => '\'',
					'x' => {
						let hi = chars.next().and_then(|c| c.to_digit(16)).ok_or(ParseStringError)?;
						let lo = chars.next().and_then(|c| c.to_digit(16)).ok_or(ParseStringError)?;
						let value = hi * 16 + lo;
						if value > 0x7f {
							return Err(ParseStringError);
						}
						char::from_u32(value).ok_or(ParseStringError)?
					},
					'u' => {
						if chars.next() != Some('{') {
							return Err(ParseStringError);
						}
						let mut value = chars.next().and_then(|c| c.to_digit(16)).ok_or(ParseStringError)?;
						let mut digits = 1;
						loop {
							match chars.next().ok_or(ParseStringError)? {
								'}' => break,
								'_' => {},
								c => {
									let digit = c.to_digit(16).ok_or(ParseStringError)?;
									digits += 1;
									if digits > 6 {
										return Err(ParseStringError);
									}
									value = value * 16 + digit;
								},
							}
						}
						char::from_u32(value).ok_or(ParseStringError)?
					},
					_ => return Err(ParseStringError),
				};
				output.push(escaped);
			},
			c => output.push(c),
		}
	}
	Ok(Cow::Owned(output))
}

/// Splits on ASCII whitespace outside double quotes.
///
/// Quotes remain in the tokens and may appear anywhere within a token.
/// Inside quotes, a backslash escapes the next character for tokenization only;
/// both characters remain in the token. Use [`string`] to decode a quoted token.
/// An unmatched quote keeps the rest of the input in the same token.
///
/// ```
/// use cvar::repl::parse::split;
///
/// let tokens: Vec<_> = split(r#"command arg1, --arg2 arg="1 2 3""#).collect();
/// assert_eq!(tokens, ["command", "arg1,", "--arg2", r#"arg="1 2 3""#]);
/// ```
#[inline]
pub fn split(line: &str) -> Split<'_> {
	Split { remainder: Some(line) }
}

/// An iterator over tokens separated by unquoted ASCII whitespace.
#[derive(Clone, Debug)]
pub struct Split<'a> {
	remainder: Option<&'a str>,
}

impl<'a> Split<'a> {
	/// Returns the unconsumed input, or `None` once the iterator is exhausted.
	///
	/// Preserves all whitespace following the last token.
	///
	/// ```
	/// use cvar::repl::parse::split;
	///
	/// let mut tokens = split("cmd  hello world");
	/// assert_eq!(tokens.next(), Some("cmd"));
	/// assert_eq!(tokens.remainder(), Some("  hello world"));
	/// tokens.by_ref().for_each(drop);
	/// assert_eq!(tokens.remainder(), None);
	/// ```
	#[inline]
	pub fn remainder(&self) -> Option<&'a str> {
		self.remainder
	}
}

impl<'a> Iterator for Split<'a> {
	type Item = &'a str;

	fn next(&mut self) -> Option<Self::Item> {
		let line = self.remainder?.trim_ascii_start();
		self.remainder = None;
		if line.is_empty() {
			return None;
		}
		let mut quoted = false;
		let mut escaped = false;
		for (index, byte) in line.bytes().enumerate() {
			if escaped {
				escaped = false;
			}
			else if quoted && byte == b'\\' {
				escaped = true;
			}
			else if byte == b'"' {
				quoted = !quoted;
			}
			else if !quoted && byte.is_ascii_whitespace() {
				let (token, remainder) = line.split_at(index);
				self.remainder = Some(remainder);
				return Some(token);
			}
		}
		Some(line)
	}
}

impl core::iter::FusedIterator for Split<'_> {}

/// Parses a command and its optional argument, separated by unquoted ASCII whitespace.
///
/// Skips leading ASCII whitespace and trims surrounding ASCII whitespace from the argument.
/// Preserves whitespace within the command token and argument.
/// Returns `None` if the input is empty or contains only ASCII whitespace.
///
/// ```
/// use cvar::repl::parse::command;
///
/// assert_eq!(command("  poke! Hello  World!\n"), Some(("poke!", Some("Hello  World!"))));
/// assert_eq!(command("poke!"), Some(("poke!", None)));
/// assert_eq!(command("  \t"), None);
/// ```
pub fn command(line: &str) -> Option<(&str, Option<&str>)> {
	let mut tokens = split(line);
	let command = tokens.next()?;
	let args = tokens.remainder().map(str::trim_ascii).filter(|s| !s.is_empty());
	Some((command, args))
}

/// Returns whether `id` appears as a token from [`split`].
///
/// ```
/// use cvar::repl::parse::flag;
///
/// assert!(flag("--verbose file.txt", "--verbose"));
/// assert!(!flag(r#""--verbose file.txt""#, "--verbose"));
/// ```
pub fn flag(line: &str, id: &str) -> bool {
	split(line).any(|s| s == id)
}

/// Returns the value of the first `id=value` token, preserving quotes.
///
/// ```
/// use cvar::repl::parse::value;
///
/// assert_eq!(value("count=3 count=4", "count"), Some("3"));
/// assert_eq!(value(r#"text="Hello World!""#, "text"), Some(r#""Hello World!""#));
/// assert_eq!(value("count=3", "missing"), None);
/// ```
pub fn value<'a>(line: &'a str, id: &str) -> Option<&'a str> {
	split(line).find_map(|s| value_of(s, id))
}

fn value_of<'a>(arg: &'a str, id: &str) -> Option<&'a str> {
	if arg.starts_with(id) && arg.as_bytes().get(id.len()) == Some(&b'=') {
		Some(&arg[id.len() + 1..])
	}
	else {
		None
	}
}

/// Returns the token at the zero-based index from [`split`].
///
/// ```
/// use cvar::repl::parse::positional;
///
/// assert_eq!(positional(r#"first "second token""#, 1), Some(r#""second token""#));
/// assert_eq!(positional("first second", 2), None);
/// ```
pub fn positional(line: &str, index: usize) -> Option<&str> {
	split(line).nth(index)
}

#[cfg(test)]
mod tests;
