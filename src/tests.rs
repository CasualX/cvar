use crate::*;

struct Foo {
	int: i32,
	float: f32,
	string: String,
}
impl Foo {
	fn action(&mut self, arg: &str, console: &mut IConsole) {
		let _ = writeln!(console, "I am {} and {}", self.string, arg);
	}
}
impl IVisit for Foo {
	fn visit_mut(&mut self, f: &mut FnMut(&mut INode)) {
		f(&mut Property::new("int", "int desc", &mut self.int, 42));
		f(&mut Property::new("float", "float desc", &mut self.float, 1.2f32));
		f(&mut Property::new("string", "string desc", &mut self.string, String::new()));
		f(&mut Action::new("action", "action desc", |args, console| self.action(args[0], console)));
	}
}
struct Root {
	before: i32,
	foo: Foo,
	after: i32,
}
impl IVisit for Root {
	fn visit_mut(&mut self, f: &mut FnMut(&mut INode)) {
		f(&mut Property::new("foo.before", "foo.before desc", &mut self.before, 1));
		f(&mut List::new("foo", "foo desc", &mut self.foo));
		f(&mut Property::new("foo.after", "foo.after desc", &mut self.after, 2));
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
	assert!(console::set(&mut root, "foo.float", "-1").is_ok());
	assert_eq!(root.foo.float, -1.0f32);
	assert!(console::set(&mut root, "foo.before", "11").is_ok());
	assert!(console::set(&mut root, "foo.after", "22").is_ok());
	assert!(console::set(&mut root, "foo.int", "parse error").is_err());
	assert!(!console::set(&mut root, "foo.list.bar", "name error").unwrap_or(true));
	assert!(!console::set(&mut root, "foo.int.bar", "list error").unwrap_or(true));
	assert!(!console::set(&mut root, "foo.action.bar", "list error").unwrap_or(true));
	assert!(!console::set(&mut root, "foo.action", "prop error").unwrap_or(true));
}
