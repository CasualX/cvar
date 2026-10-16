use super::*;
use alloc::format;
use alloc::vec::Vec;

#[test]
fn strings_decode_escapes_and_preserve_unquoted_input() {
	for (input, expected) in [
		("", ""),
		("  unquoted \\text  ", "  unquoted \\text  "),
		(r#""""#, ""),
		(r#"" padded ""#, " padded "),
		(r#""café 🦀""#, "café 🦀"),
		(r#""\0\t\n\r\\\"\'""#, "\0\t\n\r\\\"'"),
		(r#""\x00\x41\x7f""#, "\0A\x7f"),
		(r#""\u{0}\u{1_f980_}\u{10ffff}""#, "\0🦀\u{10ffff}"),
	] {
		assert_eq!(string(input).unwrap(), expected, "{input:?}");
	}
	assert!(matches!(string("raw"), Ok(Cow::Borrowed("raw"))));
	assert!(matches!(string(r#""quoted""#), Ok(Cow::Borrowed("quoted"))));
}

#[test]
fn strings_reject_incomplete_literals_and_invalid_escapes() {
	for input in [
		"\"", "\"unterminated", "\"ends in \\",
		r#""embedded"quote""#, r#""one" "two""#, "\"trailing\" ",
		r#""\q""#, r#""\x""#, r#""\x0""#, r#""\xgg""#, r#""\x80""#,
		r#""\u1234""#, r#""\u{}""#, r#""\u{_1}""#, r#""\u{gg}""#,
		r#""\u{123""#, r#""\u{d800}""#, r#""\u{110000}""#, r#""\u{0000000}""#,
	] {
		assert_eq!(string(input), Err(ParseStringError), "{input:?}");
	}
}

#[test]
fn strings_round_trip_debug_output() {
	let controls: String = (0..=0x9f).filter_map(char::from_u32).collect();
	for text in [
		"", "plain", "  whitespace\t\n", "\"quotes\" and \\slashes\\",
		"café 子 🦀", "\u{301}a\u{301}\u{200d}\u{202e}\u{feff}\u{10ffff}", &controls,
	] {
		assert_eq!(string(&format!("{text:?}")).unwrap(), text);
	}
}

#[test]
fn split_preserves_escaped_quotes_and_backslashes_in_tokens() {
	let tokens: Vec<_> = split(r#"cmd "a\" b" "c\\" tail"#).collect();
	assert_eq!(tokens, ["cmd", r#""a\" b""#, r#""c\\""#, "tail"]);
	assert_eq!(value(r#"text="a\" b" count=3"#, "count"), Some("3"));
}
