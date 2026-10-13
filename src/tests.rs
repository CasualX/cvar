use crate::*;

struct Foo {
	int: i32,
	float: f32,
	string: String,
}
impl Foo {
	fn action(&mut self, arg: &str, writer: &mut dyn IWrite) {
		let _ = writeln!(writer, "I am {} and {}", self.string, arg);
	}
}
impl IVisit for Foo {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		f(&mut Property::new("int", &mut self.int, &42));
		f(&mut Property::new("float", &mut self.float, &1.2f32));
		f(&mut Property::new("string", &mut self.string, &String::new()));
		f(&mut Action::new("action", |args, writer| self.action(args, writer)));
	}
}
struct Root {
	before: i32,
	foo: Foo,
	after: i32,
}
impl IVisit for Root {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		f(&mut Property::new("foo.before", &mut self.before, &1));
		f(&mut List::new("foo", &mut self.foo));
		f(&mut Property::new("foo.after", &mut self.after, &2));
	}
}
fn root() -> Root {
	Root {
		before: 1,
		foo: Foo {
			int: 13,
			float: -0.1f32,
			string: String::from("groot"),
		},
		after: 2,
	}
}
#[test]
fn main() {
	let mut root = root();
	let mut writer = NullWriter;
	assert!(repl::set(&mut root, "foo.float", "-1", &mut writer));
	assert_eq!(root.foo.float, -1.0f32);
	assert!(repl::set(&mut root, "foo.before", "11", &mut writer));
	assert!(repl::set(&mut root, "foo.after", "22", &mut writer));
	assert!(!repl::set(&mut root, "foo.int", "parse error", &mut writer));
	assert!(!repl::set(&mut root, "foo.list.bar", "name error", &mut writer));
	assert!(!repl::set(&mut root, "foo.int.bar", "list error", &mut writer));
	assert!(!repl::set(&mut root, "foo.action.bar", "list error", &mut writer));
	assert!(repl::set(&mut root, "foo.action", "args", &mut writer));

	assert!(repl::set_value(&mut root, "foo.int", &42, &mut writer));
	assert_eq!(repl::get_value::<i32>(&mut root, "foo.int"), Some(42));

	assert!(repl::set(&mut root, "foo.string", "any", &mut writer));
	assert_eq!(repl::get_value::<String>(&mut root, "foo.string"), Some(String::from("any")));
}

#[test]
fn poke_lists_dotted_siblings_and_real_list_children() {
	let mut root = root();
	let mut writer = String::new();
	assert!(repl::poke(&mut root, "foo", None, &mut writer));
	assert_eq!(writer, concat!(
		"foo.before is \"1\"\n",
		"foo.int is \"13\"\n",
		"foo.float is \"-0.1\"\n",
		"foo.string is \"groot\"\n",
		"foo.action\n",
		"foo.after is \"2\"\n",
	));
	assert_eq!(root.foo.string, "groot");
}

#[test]
fn poke_lists_fake_scopes_and_deeper_paths() {
	let mut value = 42;
	let mut other = true;
	let mut unrelated = 7;
	let mut root = Visit(|f| {
		f(&mut Property("scope.value", &mut value, &0));
		f(&mut Property("scope.inner.other", &mut other, &false));
		f(&mut Property("scope2.value", &mut unrelated, &0));
	});
	let mut writer = String::new();
	assert!(repl::poke(&mut root, "scope", None, &mut writer));
	assert_eq!(writer, "scope.value is \"42\"\nscope.inner.other is \"true\"\n");
	writer.clear();
	assert!(repl::poke(&mut root, "scope.inner", None, &mut writer));
	assert_eq!(writer, "scope.inner.other is \"true\"\n");
	writer.clear();
	assert!(!repl::poke(&mut root, "missing", None, &mut writer));
	assert_eq!(writer, "unknown: missing\n");
	writer.clear();
	assert!(repl::poke(&mut root, "scope.value", Some("13"), &mut writer));
	assert_eq!(writer, "scope.value is \"13\"\n");
}

#[test]
fn poke_lists_nested_real_lists_without_invoking_actions() {
	let mut value = 42;
	let mut calls = 0;
	let mut child = Visit(|f| {
		f(&mut Property("value", &mut value, &0));
		f(&mut Action("action", |_, _| calls += 1));
	});
	let mut inner = Visit(|f| f(&mut List("inner", &mut child)));
	let mut root = Visit(|f| f(&mut List("scope", &mut inner)));
	let mut writer = String::new();
	assert!(repl::poke(&mut root, "scope", Some("args"), &mut writer));
	assert_eq!(writer, "scope.inner…\nscope.inner.value is \"42\"\nscope.inner.action\n");
	writer.clear();
	assert!(repl::poke(&mut root, "scope.inner.action", Some("args"), &mut writer));
	assert!(writer.is_empty());
	assert_eq!(calls, 1);
}

#[test]
fn poke_skips_unrelated_list_visitors() {
	let mut skipped = Visit(|_| panic!("unrelated list was visited"));
	let mut skipped_inside_scope = Visit(|_| panic!("unrelated child list was visited"));
	let mut value = 42;
	let mut extra = 7;
	let mut child = Visit(|f| f(&mut Property("value", &mut value, &0)));
	let mut scope = Visit(|f| {
		f(&mut List("other", &mut skipped_inside_scope));
		f(&mut List("inner", &mut child));
	});
	let mut root = Visit(|f| {
		f(&mut List("unrelated", &mut skipped));
		f(&mut List("scope2", &mut skipped));
		f(&mut List("scope", &mut scope));
		f(&mut Property("scope.inner.extra", &mut extra, &0));
	});
	let mut writer = String::new();
	assert!(repl::poke(&mut root, "scope.inner", None, &mut writer));
	assert_eq!(writer, "scope.inner.value is \"42\"\nscope.inner.extra is \"7\"\n");
	writer.clear();
	assert!(repl::poke(&mut root, "scope.inner.value", Some("13"), &mut writer));
	assert_eq!(writer, "scope.inner.value is \"13\"\n");
	writer.clear();
	assert!(!repl::poke(&mut root, "scope.missing", None, &mut writer));
	assert_eq!(writer, "unknown: scope.missing\n");
}

#[test]
fn clamped_property_clamps_to_both_bounds() {
	let mut value = 5;
	let mut prop = ClampedProp("value", &mut value, &5, Some(&0), Some(&10));
	let mut writer = String::new();
	for (val, expected) in [(-1, 0), (11, 10), (0, 0), (10, 10), (7, 7)] {
		prop.reset();
		assert!(prop.set(&val.to_string(), &mut writer));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
		prop.reset();
		assert!(prop.set_value(&val, &mut writer));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
	}
	assert!(writer.is_empty());
}

#[test]
fn clamped_property_clamps_one_sided_bounds() {
	let mut value = 5;
	let mut writer = NullWriter;
	for (min, max, val, expected) in [
		(None, Some(10), -10, -10),
		(None, Some(10), 11, 10),
		(Some(0), None, 20, 20),
		(Some(0), None, -1, 0),
		(None, None, -20, -20),
		(None, None, 20, 20),
	] {
		let mut prop = ClampedProp("value", &mut value, &5, min.as_ref(), max.as_ref());
		prop.reset();
		assert!(prop.set(&val.to_string(), &mut writer));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
		prop.reset();
		assert!(prop.set_value(&val, &mut writer));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
	}
}

#[test]
fn clamped_property_rejects_nan_and_invalid_assignments() {
	let mut writer = String::new();
	let mut value = 0.5f32;
	let mut prop = ClampedProp("value", &mut value, &0.5, Some(&0.0), Some(&1.0));
	assert!(!prop.set("NaN", &mut writer));
	assert!(!prop.set_value(&f32::NAN, &mut writer));
	assert!(!prop.set("invalid", &mut writer));
	assert!(!prop.set_value(&42i32, &mut writer));
	assert_eq!(value, 0.5);
	assert!(!writer.is_empty());

	let mut value = 5;
	let mut prop = ClampedProp("value", &mut value, &5, Some(&10), Some(&0));
	assert!(!prop.set("-1", &mut writer));
	assert!(!prop.set_value(&11, &mut writer));
	assert_eq!(value, 5);
}
