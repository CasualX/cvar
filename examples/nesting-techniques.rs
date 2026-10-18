//! This example demonstrates more complex nesting techniques.

cvar::enum_string! {
	#[derive(Clone, Default, PartialEq)]
	enum Mode {
		#[default]
		Normal,
		Fast,
		Slow,
	}
}

#[derive(Default)]
struct Foo {
	int: i32,
	float: f32,
	string: String,
	mode: Mode,
}

// Demonstrate how to create pseudo 'on change' callbacks by aliasing the properties as actions
// Specify before or after on change by changing the order in which they are listed to the visitor
impl Foo {
	fn before_int_changed(&mut self, _args: &str, context: &mut dyn cvar::IContext) -> cvar::ActionResult {
		self.string = self.int.to_string();
		writeln!(context, "Before int is changed!");
		Ok(())
	}
	fn after_float_changed(&mut self, _args: &str, context: &mut dyn cvar::IContext) -> cvar::ActionResult {
		self.string = self.float.to_string();
		writeln!(context, "After float has changed!");
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
		f(&mut cvar::EnumProp("mode", &mut self.mode, &Mode::Normal));
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
		f(&mut cvar::BoolProp("foo.bool", &mut self.boolean, &false));
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

	prompt(&mut nested)
}

fn prompt(state: &mut dyn cvar::IVisit) {
	use std::io::{self, Write};
	loop {
		print!(">>> ");
		_ = io::stdout().flush();
		let mut line = String::new();
		if io::stdin().read_line(&mut line).is_err() || line.is_empty() {
			break;
		}
		let (path, args) = cvar::repl::parse::command(&line).unwrap_or(("", None));
		let context: &mut dyn cvar::IContext = &mut cvar::IoWriter::stdout();
		if let Ok(false) = cvar::repl::poke(state, path, args, context) {
			context.write_error(&format_args!("path {path:?} does not exist"));
		}
	}
}
