/*!
This example demonstrates more complex nesting techniques.
!*/

#[derive(Default)]
struct Foo {
	int: i32,
	float: f32,
	string: String,
}
// Demonstrate how to create pseudo 'on change' callbacks by aliasing the properties as actions
// Specify before or after on change by changing the order in which they are listed to the visitor
impl Foo {
	fn before_int_changed(&mut self, _args: &[&str], console: &mut dyn cvar::IConsole) {
		self.string = self.int.to_string();
		let _ = writeln!(console, "Before int has changed!");
	}
	fn after_float_changed(&mut self, _args: &[&str], console: &mut dyn cvar::IConsole) {
		self.string = self.float.to_string();
		let _ = writeln!(console, "After float has changed!");
	}
}
impl cvar::IVisit for Foo {
	fn visit_mut(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Action("int", "", |args, console| self.before_int_changed(args, console)));
		f(&mut cvar::Property("int", "", &mut self.int, 0));
		f(&mut cvar::Property("float", "", &mut self.float, 0.0));
		f(&mut cvar::Action("float", "", |args, console| self.after_float_changed(args, console)));
		f(&mut cvar::Property("string", "", &mut self.string, String::new()));
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
	fn visit_mut(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Property("foo.bool", "", &mut self.boolean, false));
		f(&mut cvar::List("foo", "", &mut self.foo));
	}
}

fn main() {
	let mut nested = Nested::default();

	// This property appears nested but is set in the parent context
	// Note that `set` won't invoke actions and therefore change callbacks won't be called
	// To invoke change callbacks implement this set operation yourself (see below)
	cvar::console::set(&mut nested, "foo.bool", "true").unwrap();
	assert!(nested.boolean);

	println!("Hit enter to list all the cvars and their values.");
	println!("Assign value to cvar with `<name> <value>`.");

	loop {
		// Read input from stdin
		let mut line = String::new();
		if read_line(&mut line) {
			break;
		}

		// Crude command line parsing
		let args: Vec<&str> = line.split_whitespace().collect();

		// Print the tree of props if empty
		if args.is_empty() {
			cvar::console::walk(&mut nested, |path, node| {
				match node.as_node_mut() {
					cvar::NodeMut::Prop(prop) => {
						println!("{} `{}`", path, prop.get());
					},
					cvar::NodeMut::List(_list) => (),
					cvar::NodeMut::Action(_act) => {
						println!("{}", path);
					},
				}
			});
			continue;
		}

		// Find the node the user wants to interact with
		let path = args[0];
		let args = &args[1..];
		if !cvar::console::find(&mut nested, path, |node| {
			match node.as_node_mut() {
				cvar::NodeMut::Prop(prop) => {
					// If we passed any arguments, try to set the value
					if args.len() >= 1 {
						if let Err(err) = prop.set(args[0]) {
							println!("Cannot parse `{}`: {}.", args[0], err);
						}
					}
					// In any case print the value the prop currently has
					println!("{} `{}`", path, prop.get());
				},
				cvar::NodeMut::Action(act) => {
					// Redirect to stdout
					let mut console = cvar::IoConsole::stdout();
					act.invoke(args, &mut console);
				},
				cvar::NodeMut::List(_) => {},
			}
		}) {
			println!("Cannot find `{}`", path);
		}
	}
}

// Reads a line from stdin and return true if there was a break
pub fn read_line(line: &mut String) -> bool {
	use std::io;
	print!(">>> ");
	let _ = io::Write::flush(&mut io::stdout());
	return io::stdin().read_line(line).is_err() || line.is_empty();
}
