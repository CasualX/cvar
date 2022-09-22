/*!
This example demonstrates how properties can be created and destroyed at runtime.
*/

#[derive(Debug, Default)]
struct RuntimeProps {
	// Store the list of runtime properties somewhere
	props: Vec<Box<dyn cvar::IProperty>>,
}

impl RuntimeProps {
	// Action to create new properties
	fn create(&mut self, args: &str, writer: &mut dyn cvar::IWrite) {
		// Crude argument parsing
		let args = args.trim();
		let first = args.split_ascii_whitespace().next().unwrap_or("");
		let args = args[first.len()..].trim_start();
		let second = args.split_ascii_whitespace().next().unwrap_or("");
		let third = args[second.len()..].trim_start();
		if first.len() == 0 {
			let _ = writeln!(writer, "Invalid arguments! expecting <type> <name> <value>");
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
				let _ = writeln!(writer, "Invalid type! supports string, int or float");
			},
		}
	}
	// Action to remove properties
	fn destroy(&mut self, args: &str, writer: &mut dyn cvar::IWrite) {
		if args.len() == 0 {
			let _ = writeln!(writer, "Invalid arguments! expecting the name of the property to remove");
			return;
		}
		self.props.retain(|prop| prop.name() != args);
	}
}

impl cvar::IVisit for RuntimeProps {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Action("create!", |args, writer| self.create(args, writer)));
		f(&mut cvar::Action("destroy!", |args, writer| self.destroy(args, writer)));
		for prop in &mut self.props {
			f(prop.as_inode());
		}
	}
}

fn main() {
	let mut runtime_props = RuntimeProps::default();

	// Create some runtime props
	let mut writer = String::new();
	cvar::console::invoke(&mut runtime_props, "create!", "float f 3.141592", &mut writer);
	cvar::console::invoke(&mut runtime_props, "create!", "string s Hello World!", &mut writer);
	cvar::console::invoke(&mut runtime_props, "create!", "int i 42", &mut writer);

	// Inspect the underlying props
	assert_eq!(runtime_props.props.len(), 3);
	assert_eq!(runtime_props.props[0].get_value().to_string(), "3.141592");
	assert_eq!(runtime_props.props[1].get_value().to_string(), "Hello World!");
	assert_eq!(runtime_props.props[2].get_value().to_string(), "42");

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
		let (path, args) = split_line(&line);
		cvar::console::poke(&mut runtime_props, path, args, &mut cvar::IoWriter::stdout());
	}
}

// Reads a line from stdin and return true if there was a break
pub fn read_line(line: &mut String) -> bool {
	use std::io;
	print!(">>> ");
	let _ = io::Write::flush(&mut io::stdout());
	return io::stdin().read_line(line).is_err() || line.is_empty();
}

pub fn split_line(line: &str) -> (&str, Option<&str>) {
	let line = line.trim_start();
	let path = line.split_ascii_whitespace().next().unwrap_or("");
	let args = &line[path.len()..].trim();
	(path, if args.len() == 0 { None } else { Some(args) })
}
