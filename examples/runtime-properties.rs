/*!
This example demonstrates how properties can be created and destroyed at runtime.
!*/

#[derive(Debug, Default)]
struct RuntimeProps {
	// Store the list of runtime properties somewhere
	props: Vec<Box<dyn cvar::IProperty>>,
}
impl RuntimeProps {
	// Action to create new properties
	fn create(&mut self, args: &str, console: &mut dyn cvar::IConsole) {
		// Crude argument parsing
		let args = args.trim();
		let first = args.split_ascii_whitespace().next().unwrap_or("");
		let args = args[first.len()..].trim_start();
		let second = args.split_ascii_whitespace().next().unwrap_or("");
		let third = args[second.len()..].trim_start();
		if first.len() == 0 {
			let _ = writeln!(console, "Invalid arguments! expecting <type> <name> <value>");
			return;
		}
		match first {
			"string" => {
				let prop = cvar::OwnedProp(second.into(), String::from(third), String::from(third));
				self.props.push(Box::new(prop));
			},
			"int" => {
				let value: i32 = third.parse().unwrap();
				let prop = cvar::OwnedProp(second.into(), value, value);
				self.props.push(Box::new(prop));
			},
			"float" => {
				let value: f32 = third.parse().unwrap();
				let prop = cvar::OwnedProp(second.into(), value, value);
				self.props.push(Box::new(prop));
			},
			_ => {
				let _ = writeln!(console, "Invalid type! supports string, int or float");
			},
		}
	}
	// Action to remove properties
	fn destroy(&mut self, args: &str, console: &mut dyn cvar::IConsole) {
		if args.len() == 0 {
			let _ = writeln!(console, "Invalid arguments! expecting the name of the property to remove");
			return;
		}
		self.props.retain(|prop| prop.name() != args);
	}
}
impl cvar::IVisit for RuntimeProps {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Action("create!", |args, console| self.create(args, console)));
		f(&mut cvar::Action("destroy!", |args, console| self.destroy(args, console)));
		for prop in &mut self.props {
			f(prop.as_inode());
		}
	}
}

fn main() {
	let mut runtime_props = RuntimeProps::default();

	// Create some runtime props
	let mut output = String::new();
	cvar::console::invoke(&mut runtime_props, "create!", "float f 3.141592", &mut output);
	cvar::console::invoke(&mut runtime_props, "create!", "string s Hello World!", &mut output);
	cvar::console::invoke(&mut runtime_props, "create!", "int i 42", &mut output);

	// Inspect the underlying props
	assert_eq!(runtime_props.props.len(), 3);
	assert_eq!(runtime_props.props[0].get(), "3.141592");
	assert_eq!(runtime_props.props[1].get(), "Hello World!");
	assert_eq!(runtime_props.props[2].get(), "42");

	println!("Hit enter to list all the cvars and their values.");
	println!("Assign value to cvar with `<name> <value>`.");
	println!("Create new cvars with `create! <type> <name> <value>`.");
	println!("Destroy the cvars with `destroy! <name>.");

	loop {
		// Read input from stdin
		let mut line = String::new();
		if read_line(&mut line) {
			break;
		}

		// Crude command line parsing
		let line = line.trim();
		match line.split_ascii_whitespace().next() {
			// Find the node the user wants to interact with
			Some(path) => {
				let args = line[path.len()..].trim_start();
				if !cvar::console::find(&mut runtime_props, path, |node| {
					match node.as_node() {
						cvar::Node::Prop(prop) => {
							// If we passed any arguments, try to set the value
							if args.len() > 0 {
								if let Err(err) = prop.set(args) {
									println!("Cannot parse `{}`: {}.", args, err);
								}
							}
							// In any case print the value the prop currently has
							println!("{} `{}`", path, prop.get());
						},
						cvar::Node::Action(act) => {
							// Redirect output to stdout
							let mut console = cvar::IoConsole::stdout();
							act.invoke(args, &mut console);
						},
						cvar::Node::List(_) => {},
					}
				}) {
					println!("Cannot find `{}`", path);
				}
			},
			None => {
				// Print the tree of props if empty
				cvar::console::walk(&mut runtime_props, |path, node| {
					match node.as_node() {
						cvar::Node::Prop(prop) => {
							println!("{} `{}`", path, prop.get());
						},
						cvar::Node::List(_list) => (),
						cvar::Node::Action(_act) => {
							println!("{}", path);
						},
					}
				});
			},
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
