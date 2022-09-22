/*!
The example from the readme.
*/

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
	let mut writer = String::new();

	let mut program_state = ProgramState {
		number: 42,
		text: String::new(),
	};

	assert_eq!(cvar::console::get(&mut program_state, "number").unwrap(), "42");

	cvar::console::set(&mut program_state, "number", "13", &mut writer);
	assert_eq!(program_state.number, 13);

	cvar::console::invoke(&mut program_state, "poke!", "the value is", &mut writer);
	assert_eq!(program_state.text, "the value is: 13");
}
