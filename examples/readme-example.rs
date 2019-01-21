/*!
The example from the readme.
!*/

pub struct ProgramState {
	number: i32,
	text: String,
}
impl ProgramState {
	pub fn poke(&mut self, arg: &str) {
		self.text = format!("{}: {}", arg, self.number);
	}
}

impl cvar::IVisit for ProgramState {
	fn visit_mut(&mut self, f: &mut FnMut(&mut cvar::INode)) {
		f(&mut cvar::Property("number", "this is a description", &mut self.number, 42));
		f(&mut cvar::Property("text", "another description", &mut self.text, String::new()));
		f(&mut cvar::Action("poke!", "change the state", |args, _console| self.poke(args[0])));
	}
}

fn main() {
	let mut program_state = ProgramState {
		number: 42,
		text: String::new(),
	};

	assert_eq!(cvar::console::get(&mut program_state, "number").unwrap(), "42");

	cvar::console::set(&mut program_state, "number", "13").unwrap();
	assert_eq!(program_state.number, 13);

	let mut console = String::new();
	cvar::console::invoke(&mut program_state, "poke!", &["the value is"], &mut console);
	assert_eq!(program_state.text, "the value is: 13");
}
