//! This example demonstrates how properties can be created and destroyed at runtime.

#[derive(Debug, Default)]
struct RuntimeProps {
	// Store the list of runtime properties somewhere
	props: Vec<Box<dyn cvar::IProperty>>,
}

impl RuntimeProps {
	// Action to create new properties
	fn create(&mut self, args: &str, context: &mut dyn cvar::IContext) -> cvar::ActionResult {
		let mut args = cvar::repl::parse::split(args.trim_ascii());
		let (Some(first), Some(second)) = (args.next(), args.next()) else {
			context.write_error("Invalid arguments! expecting <type> <name> <value>");
			return Err(cvar::ActionError);
		};
		let third = args.remainder().unwrap_or("").trim_ascii_start();
		match first {
			"string" => {
				let prop = cvar::OwnedProp(second.into(), String::from(third), String::from(third));
				self.props.push(Box::new(prop));
			},
			"int" => {
				let value: i32 = match third.parse() {
					Ok(value) => value,
					Err(err) => {
						context.write_error(&format_args!("Invalid int value: {err}"));
						return Err(cvar::ActionError);
					},
				};
				let prop = cvar::OwnedProp(second.into(), value, value);
				self.props.push(Box::new(prop));
			},
			"float" => {
				let value: f32 = match third.parse() {
					Ok(value) => value,
					Err(err) => {
						context.write_error(&format_args!("Invalid float value: {err}"));
						return Err(cvar::ActionError);
					},
				};
				let prop = cvar::OwnedProp(second.into(), value, value);
				self.props.push(Box::new(prop));
			},
			_ => {
				context.write_error("Invalid type! supports string, int or float");
				return Err(cvar::ActionError);
			},
		}
		Ok(())
	}
	// Action to remove properties
	fn destroy(&mut self, args: &str, context: &mut dyn cvar::IContext) -> cvar::ActionResult {
		let Some(name) = cvar::repl::parse::positional(args, 0) else {
			context.write_error("Invalid arguments! expecting the name of the property to remove");
			return Err(cvar::ActionError);
		};
		self.props.retain(|prop| prop.name() != name);
		Ok(())
	}
}

impl cvar::IVisit for RuntimeProps {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Action("create!", |args, context| self.create(args, context)));
		f(&mut cvar::Action("destroy!", |args, context| self.destroy(args, context)));
		for prop in &mut self.props {
			f(prop.as_mut());
		}
	}
}

fn main() {
	let mut runtime_props = RuntimeProps::default();

	// Create some runtime props
	let mut context = String::new();
	cvar::repl::invoke(&mut runtime_props, "create!", "float f 3.141592", &mut context).unwrap();
	cvar::repl::invoke(&mut runtime_props, "create!", "string s Hello World!", &mut context).unwrap();
	cvar::repl::invoke(&mut runtime_props, "create!", "int i 42", &mut context).unwrap();

	// Inspect the underlying props
	assert_eq!(runtime_props.props.len(), 3);
	assert_eq!(runtime_props.props[0].get_value().to_string(), "3.141592");
	assert_eq!(runtime_props.props[1].get_value().to_string(), "Hello World!");
	assert_eq!(runtime_props.props[2].get_value().to_string(), "42");

	println!("Hit enter to list all the cvars and their values.");
	println!("Assign value to cvar with `<name> <value>`.");
	println!("Create new cvars with `create! <type> <name> <value>`.");
	println!("Destroy the cvars with `destroy! <name>`.");

	loop {
		// Read input from stdin
		let mut line = String::new();
		if read_line(&mut line) {
			break;
		}

		let (path, args) = cvar::repl::parse::command(&line).unwrap_or(("", None));
		_ = cvar::repl::poke(&mut runtime_props, path, args, &mut cvar::IoWriter::stdout());
	}
}

// Reads a line from stdin and return true if there was a break
pub fn read_line(line: &mut String) -> bool {
	use std::io;
	print!(">>> ");
	_ = io::Write::flush(&mut io::stdout());
	io::stdin().read_line(line).is_err() || line.is_empty()
}

#[test]
fn create_rejects_invalid_input() {
	let mut props = RuntimeProps::default();
	for args in ["", "int", "int value", "int value invalid", "float value invalid", "string"] {
		let mut context = String::new();
		assert_eq!(cvar::repl::invoke(&mut props, "create!", args, &mut context), Err(cvar::ActionError));
		assert!(context.starts_with("error: "));
		assert!(context.ends_with('\n'));
		assert!(props.props.is_empty());
	}
	let mut context = String::new();
	assert_eq!(cvar::repl::invoke(&mut props, "create!", "int value 42", &mut context), Ok(()));
	assert_eq!(cvar::repl::get_value::<i32>(&mut props, "value"), Some(42));
}
