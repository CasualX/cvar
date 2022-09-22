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
	assert!(matches!(console::set(&mut root, "foo.float", "-1", &mut writer), true));
	assert_eq!(root.foo.float, -1.0f32);
	assert!(matches!(console::set(&mut root, "foo.before", "11", &mut writer), true));
	assert!(matches!(console::set(&mut root, "foo.after", "22", &mut writer), true));
	assert!(matches!(console::set(&mut root, "foo.int", "parse error", &mut writer), false));
	assert!(matches!(console::set(&mut root, "foo.list.bar", "name error", &mut writer), false));
	assert!(matches!(console::set(&mut root, "foo.int.bar", "list error", &mut writer), false));
	assert!(matches!(console::set(&mut root, "foo.action.bar", "list error", &mut writer), false));
	assert!(matches!(console::set(&mut root, "foo.action", "prop error", &mut writer), false));

	assert!(console::set_value(&mut root, "foo.int", &42, &mut writer));
	assert_eq!(console::get_value::<i32>(&mut root, "foo.int"), Some(42));

	assert!(console::set(&mut root, "foo.string", "any", &mut writer));
	assert_eq!(console::get_value::<String>(&mut root, "foo.string"), Some(String::from("any")));
}
