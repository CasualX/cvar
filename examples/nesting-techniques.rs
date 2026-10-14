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
	fn before_int_changed(&mut self, _args: &str, context: &mut dyn cvar::IContext) -> cvar::ActionResult {
		self.string = self.int.to_string();
		writeln!(context, "Before int is changed!")?;
		Ok(())
	}
	fn after_float_changed(&mut self, _args: &str, context: &mut dyn cvar::IContext) -> cvar::ActionResult {
		self.string = self.float.to_string();
		writeln!(context, "After float has changed!")?;
		Ok(())
	}
}

impl cvar::IVisit for Foo {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::OnChange("int", |args, context| self.before_int_changed(args, context)));
		f(&mut cvar::Property("int", &mut self.int, &0));
		f(&mut cvar::Property("float", &mut self.float, &0.0));
		f(&mut cvar::OnChange("float", |args, context| self.after_float_changed(args, context)));
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
	cvar::repl::set_value(&mut nested, "foo.bool", &true, &mut cvar::NullContext).unwrap();
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
		_ = cvar::repl::poke(&mut nested, path, args, &mut cvar::IoWriter::stdout());
	}
}

// Reads a line from stdin and return true if there was a break
pub fn read_line(line: &mut String) -> bool {
	use std::io;
	print!(">>> ");
	_ = io::Write::flush(&mut io::stdout());
	io::stdin().read_line(line).is_err() || line.is_empty()
}
