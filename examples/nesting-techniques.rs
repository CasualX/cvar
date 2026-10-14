//! This example demonstrates more complex nesting techniques.

#[derive(Default)]
struct Foo {
	int: i32,
	float: f32,
	string: String,
}

// Demonstrate how to create pseudo 'on change' callbacks by aliasing the properties as actions
// Specify before or after on change by changing the order in which they are listed to the visitor
impl Foo {
	fn before_int_changed(&mut self, _args: &str, writer: &mut dyn cvar::IWrite) {
		self.string = self.int.to_string();
		let _ = writeln!(writer, "Before int is changed!");
	}
	fn after_float_changed(&mut self, _args: &str, writer: &mut dyn cvar::IWrite) {
		self.string = self.float.to_string();
		let _ = writeln!(writer, "After float has changed!");
	}
}

impl cvar::IVisit for Foo {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Action("int", |args, writer| self.before_int_changed(args, writer)));
		f(&mut cvar::Property("int", &mut self.int, &0));
		f(&mut cvar::Property("float", &mut self.float, &0.0));
		f(&mut cvar::Action("float", |args, writer| self.after_float_changed(args, writer)));
		f(&mut cvar::Property("string", &mut self.string, &String::new()));
	}
}

// Demonstrate how to create pseudo hierarchy allowing to inject nodes in a deeper nested namespace
// It is not possible to inject a node in a parent, this would also clash with Rust's borrowing rules
#[derive(Default)]
struct Nested {
	boolean: bool,
	foo: Foo,
}

impl cvar::IVisit for Nested {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Property("foo.bool", &mut self.boolean, &false));
		f(&mut cvar::List("foo", &mut self.foo));
	}
}

fn main() {
	let mut nested = Nested::default();

	// This property appears nested but is set in the parent context
	cvar::repl::set_value(&mut nested, "foo.bool", &true, &mut cvar::NullWriter);
	assert!(nested.boolean);

	println!("Hit enter to list all the cvars and their values.");
	println!("Assign value to cvar with `<name> <value>`.");

	loop {
		// Read input from stdin
		let mut line = String::new();
		if read_line(&mut line) {
			break;
		}

		let (path, args) = cvar::repl::parse::command(&line).unwrap_or(("", None));
		cvar::repl::poke(&mut nested, path, args, &mut cvar::IoWriter::stdout());
	}
}

// Reads a line from stdin and return true if there was a break
pub fn read_line(line: &mut String) -> bool {
	use std::io;
	print!(">>> ");
	let _ = io::Write::flush(&mut io::stdout());
	io::stdin().read_line(line).is_err() || line.is_empty()
}
