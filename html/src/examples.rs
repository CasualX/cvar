// Duplicated from the three native examples; no terminal dependencies.

pub struct ProgramState {
	pub(super) number: i32,
	pub(super) text: String,
}

impl ProgramState {
	pub fn poke(&mut self, args: &str) {
		self.text = format!("{}: {}", args, self.number);
	}
}

impl cvar::IVisit for ProgramState {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::Property("number", &mut self.number, &42));
		f(&mut cvar::Property("text", &mut self.text, &String::new()));
		f(&mut cvar::Action("poke!", |args, _context| { self.poke(args); Ok(()) }));
	}
}

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
pub(super) struct Nested {
	boolean: bool,
	foo: Foo,
}

impl cvar::IVisit for Nested {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		f(&mut cvar::BoolProp("foo.bool", &mut self.boolean, &false));
		f(&mut cvar::List("foo", &mut self.foo));
	}
}

#[derive(Debug, Default)]
pub(super) struct RuntimeProps {
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
