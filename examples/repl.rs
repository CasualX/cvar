extern crate cvar;

use ::std::cell::{Cell, RefCell};
use ::std::io::{self, Write};

//----------------------------------------------------------------
// Basic setup

struct Foo {
	secure: Cell<bool>,
	url: RefCell<String>,
}

impl Foo {
	fn display(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		let url = self.url.borrow();
		let url = &*url;
		writeln!(ctx.write, "The specified URL is {}.", url).ok();
		Ok(())
	}
}

impl cvar::IVisit for Foo {
	fn visit(&self, f: &mut FnMut(cvar::Node)) {
		f(From::from(&cvar::Property::new("secure", "secure description", &self.secure, false)));
		f(From::from(&cvar::Property::new("url", "url description", &self.url, "")));
		f(From::from(&cvar::Action::new("display", "display description", |ctx| self.display(ctx))));
	}
}

struct Dynamic {
	props: RefCell<Vec<Box<cvar::IProperty>>>,
}
impl Dynamic {
	fn create(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		let mut args = ctx.args.split_whitespace();
		if let (Some(ty), Some(name), Some(def), None) = (args.next(), args.next(), args.next(), args.next()) {
			let prop: Box<cvar::IProperty> = match ty {
				"string" => {
					Box::new(cvar::Property::new(String::from(name), "dynamic string", RefCell::new(String::from(def)), String::from(def)))
				},
				"int" => {
					let val: i32 = try!(def.parse());
					Box::new(cvar::Property::new(String::from(name), "dynamic int", Cell::new(val), val))
				},
				"float" => {
					let val: f32 = try!(def.parse());
					Box::new(cvar::Property::new(String::from(name), "dynamic float", Cell::new(val), val))
				},
				ty => {
					println!("Unknown type {}, only string, int, float supported", ty);
					// You are supposed to return an error...
					return Ok(());
				},
			};
			self.props.borrow_mut().push(prop);
			Ok(())
		}
		else {
			println!("Expecting <type> <name> <default>");
			Ok(())
		}
	}
}
impl cvar::IVisit for Dynamic {
	fn visit(&self, f: &mut FnMut(cvar::Node)) {
		f(From::from(&cvar::Action::new("create", "create description", |ctx| self.create(ctx))));
		for it in &*self.props.borrow() {
			f(From::from(&**it));
		}
	}
}

struct Root {
	foo: Foo,
	dyn: Dynamic,
}
impl Root {
	fn props(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		cvar::console::walk(self, |walk, node| {
			if let cvar::Node::Prop(prop) = node {
				writeln!(ctx.write,
					"{:24} {:8} {:8} - {}",
					walk.id(prop.name()),
					format!("{:?}", prop.state()),
					format!("{:?}", prop.get()),
					prop.description()
				).ok();
			};
		});
		Ok(())
	}
	fn print(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		try!(writeln!(ctx.write, "{:#?}", self as &cvar::IVisit));
		Ok(())
	}
	fn reset(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		if ctx.args.len() == 0 {
			try!(writeln!(ctx.write, "reset_all: {:?}", cvar::console::reset_all(self)))
		}
		else {
			try!(writeln!(ctx.write, "reset {}: {:?}", ctx.args, cvar::console::reset(self, ctx.args)));
		}
		Ok(())
	}
	fn help(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		Ok(try!(write!(ctx.write, "{}", HELP_TEXT)))
	}
	fn end_of_demo(&self, ctx: &mut cvar::Context) -> cvar::BoxResult<()> {
		Ok(try!(writeln!(ctx.write, "You've reached the end of the demo!")))
	}
}
impl cvar::IVisit for Root {
	fn visit(&self, f: &mut FnMut(cvar::Node)) {
		use cvar::{List, Action};
		f(From::from(&List::new("foo", "foo description", &self.foo)));
		f(From::from(&List::new("dyn", "dynamic description", &self.dyn)));
		f(From::from(&Action::new("props", "list all properties", |ctx| self.props(ctx))));
		f(From::from(&Action::new("print", "debug print", |ctx| self.print(ctx))));
		f(From::from(&Action::new("reset", "reset configuration", |ctx| self.reset(ctx))));
		f(From::from(&Action::new("help", "display help text", |ctx| self.help(ctx))));
		f(From::from(&Action::new("end_of_demo", "display the end of demo text", |ctx| self.end_of_demo(ctx))));
	}
}

//----------------------------------------------------------------
// Interaction

static HELP_TEXT: &'static str = "\
Syntax:

Get value:  <id>
Set value:  <id> <val>
Print list: <id>
Invoke:     <id> <args>

Press enter when empty to advance the demo!

";

fn main() {
	// Your root must be instantiated.
	let root = Root {
		foo: Foo {
			secure: Cell::new(false),
			url: RefCell::default(),
		},
		dyn: Dynamic {
			props: RefCell::new(Vec::new()),
		},
	};

	print!("{}", HELP_TEXT);

	let mut demo = vec![
		"print",
		"foo.secure",
		"foo.url https://www.google.com/",
		"foo.secure true",
		"foo.display",
		"dyn.create",
		"dyn.create string hello World!",
		"dyn.hello",
		"dyn.hello <subject name here>",
		"dyn.hello",
		"dyn.create int int 42",
		"dyn.create float float 3.141592",
		"props",
		"reset",
		"props",
		"end_of_demo",
	].into_iter();
	
	loop {
		print!(">>> ");
		io::stdout().flush().ok();
		// Read input from stdin
		let mut line = String::new();
		if io::stdin().read_line(&mut line).is_err() {
			break;
		}
		// Not sure how to handle ctrl-c events, Rust’s read_line is a bit weird in this regard
		// I basically get an empty string as opposed to a newline when you just press enter.
		if line.len() == 0 {
			break;
		}
		// Demo time!
		let line = if line.trim().len() == 0 {
			if let Some(next) = demo.next() {
				println!("demo >>> {}", next);
				next
			}
			else {
				continue;
			}
		}
		else {
			&line
		};
		let mut args = line.split_whitespace();
		if let Some(id) = args.next() {
			// Extract all remaining arguments...
			let tail = args.next().map(|arg| {
				let split_at = arg.as_ptr() as usize - line.as_ptr() as usize;
				line[split_at..].trim()
			});
			let err = cvar::console::find(&root, id, |node| {
				match node {
					cvar::Node::Prop(prop) => {
						if let Some(val) = tail {
							println!("{:?}", try!(prop.set(val)));
						}
						else {
							println!("{:?}", prop.get());
						}
					},
					cvar::Node::List(list) => {
						println!("{:?}", list);
					},
					cvar::Node::Action(act) => {
						println!("{:?}", try!(act.invoke(&mut cvar::Context::new(tail.unwrap_or(""), &mut io::stdout()))));
					},
				};
				Ok(())
			});
			if let Err(err) = err {
				writeln!(io::stderr(), "{:#?}", err).ok();
			}
		}
	};
}
