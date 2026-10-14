//! The ProgramState example from the readme.

pub struct ProgramState {
	number: i32,
	text: String,
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
		f(&mut cvar::Action("poke!", |args, _writer| self.poke(args)));
	}
}

fn main() {
	use std::io::{self, Write};

	let mut program_state = ProgramState {
		number: 42,
		text: String::new(),
	};
	println!("Hit enter to list the properties and actions.");
	println!("Read with <name>, assign with <name> <value>, invoke with poke! <text>.");

	loop {
		print!(">>> ");
		let _ = io::stdout().flush();
		let mut line = String::new();
		if io::stdin().read_line(&mut line).is_err() || line.is_empty() {
			break;
		}
		let (path, args) = cvar::repl::parse::command(&line).unwrap_or(("", None));
		cvar::repl::poke(&mut program_state, path, args, &mut cvar::IoWriter::stdout());
	}
}
