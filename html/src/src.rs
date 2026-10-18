mod examples;

use std::cell::RefCell;
use examples::*;

enum Demo {
	Program(ProgramState),
	Nested(Nested),
	Runtime(RuntimeProps),
}

impl Demo {
	fn new(id: u32) -> Result<Self, String> {
		match id {
			0 => Ok(Self::Program(ProgramState { number: 42, text: String::new() })),
			1 => {
				let mut state = Nested::default();
				cvar::repl::set_value(&mut state, "foo.bool", &true, &mut cvar::NullContext).unwrap();
				Ok(Self::Nested(state))
			},
			2 => {
				let mut state = RuntimeProps::default();
				for args in ["float f 3.141592", "string s Hello World!", "int i 42"] {
					cvar::repl::invoke(&mut state, "create!", args, &mut cvar::NullContext).unwrap();
				}
				Ok(Self::Runtime(state))
			},
			_ => Err(format!("Unknown example: {id}")),
		}
	}

	fn visitor(&mut self) -> &mut dyn cvar::IVisit {
		match self {
			Self::Program(state) => state,
			Self::Nested(state) => state,
			Self::Runtime(state) => state,
		}
	}
}

thread_local! {
	static STATE: RefCell<Demo> = RefCell::new(Demo::new(0).unwrap());
}

#[link(wasm_import_module = "env")]
unsafe extern "C" {
	fn resultTextLines(s: *const u8, len: usize);
	fn resultError(s: *const u8, len: usize);
}

fn respond(result: Result<String, String>) {
	// JS copies these bytes synchronously; it must not retain the pointer.
	match result {
		Ok(value) => unsafe { resultTextLines(value.as_ptr(), value.len()) },
		Err(error) => unsafe { resultError(error.as_ptr(), error.len()) },
	}
}

/// Allocated input is zero-initialized, then filled by JS and freed after the call.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
	Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
}

/// The pointer and length must match a live allocation returned by `alloc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
	unsafe { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len))); }
}

// Input pointers must refer to initialized bytes in a live `alloc` allocation.
unsafe fn input<'a>(ptr: *const u8, len: usize) -> Result<&'a str, String> {
	std::str::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) })
		.map_err(|error| error.to_string())
}

fn output(state: &mut Demo, line: &str) -> Result<String, String> {
	let mut output = String::new();
	let (path, args) = cvar::repl::parse::command(line).unwrap_or(("", None));
	match cvar::repl::poke(state.visitor(), path, args, &mut output) {
		Ok(true) => Ok(output),
		Ok(false) => Err(format!("path {path:?} does not exist")),
		Err(_) => Err(output),
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn select_example(id: u32) {
	respond(Demo::new(id).and_then(|demo| STATE.with(|state| {
		*state.borrow_mut() = demo;
		output(&mut state.borrow_mut(), "")
	})));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execute(ptr: *const u8, len: usize) {
	respond(unsafe { input(ptr, len) }.and_then(|line| {
		STATE.with(|state| output(&mut state.borrow_mut(), line))
	}));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn complete(ptr: *const u8, len: usize) {
	respond(unsafe { input(ptr, len) }.map(|partial| STATE.with(|state| {
		let mut paths = cvar::repl::paths(state.borrow_mut().visitor(), partial);
		let mut unique = Vec::new();
		for path in paths.drain(..) {
			if !unique.contains(&path) { unique.push(path); }
		}
		unique.join("\n")
	})));
}
