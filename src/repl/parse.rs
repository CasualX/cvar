//! Simple command and argument parsing.

/// Splits on ASCII whitespace outside double quotes.
///
/// Quotes remain in the tokens and may appear anywhere within a token.
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
		for (index, byte) in line.bytes().enumerate() {
			if byte == b'"' {
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
