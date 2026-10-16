use crate::*;

struct Foo {
	int: i32,
	float: f32,
	string: String,
}
impl IVisit for Foo {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		f(&mut Property("int", &mut self.int, &42));
		f(&mut Property("float", &mut self.float, &1.2));
		f(&mut Property("string", &mut self.string, &String::new()));
		f(&mut Action("action", |args, context| {
			writeln!(context, "I am {} and {args}", self.string);
			Ok(())
		}));
	}
}
struct Root {
	before: i32,
	foo: Foo,
	after: i32,
}
impl IVisit for Root {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		f(&mut Property("foo.before", &mut self.before, &1));
		f(&mut List("foo", &mut self.foo));
		f(&mut Property("foo.after", &mut self.after, &2));
	}
}
fn root() -> Root {
	Root {
		before: 1,
		foo: Foo { int: 13, float: -0.1, string: String::from("groot") },
		after: 2,
	}
}

#[test]
fn repl_reads_writes_and_reports_errors() {
	let mut root = root();
	let mut context = String::new();
	assert_eq!(repl::set(&mut root, "foo.float", "-1", &mut context), Ok(true));
	assert_eq!(root.foo.float, -1.0);
	assert_eq!(repl::set_value(&mut root, "foo.int", &42, &mut context), Ok(true));
	assert_eq!(repl::get_value::<i32>(&mut root, "foo.int"), Some(42));
	assert_eq!(repl::set(&mut root, "foo.string", "hello", &mut context), Ok(true));
	assert_eq!(repl::get(&mut root, "foo.string"), Some(String::from("hello")));
	assert_eq!(repl::invoke(&mut root, "foo.action", "world", &mut context), Ok(true));
	assert_eq!(context, "I am hello and world\n");

	context.clear();
	assert_eq!(repl::set(&mut root, "foo.int", "invalid", &mut context), Err(ActionError));
	assert_eq!(root.foo.int, 42);
	assert!(context.starts_with("error: "));
	assert_eq!(repl::set_value(&mut root, "foo.int", &true, &mut context), Err(ActionError));
	assert!(context.contains("error: mismatched types"));
	assert_eq!(repl::set(&mut root, "missing", "42", &mut context), Ok(false));
	assert_eq!(repl::get(&mut root, "missing"), None);
	assert_eq!(repl::invoke(&mut root, "foo.int", "", &mut context), Ok(false));
}

#[test]
fn missing_matches_return_false_without_errors() {
	let mut root = root();
	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "missing", None, &mut context), Ok(false));
	assert_eq!(repl::poke(&mut root, "missing", Some("42"), &mut context), Ok(false));
	assert_eq!(repl::set(&mut root, "missing", "42", &mut context), Ok(false));
	assert_eq!(repl::set_value(&mut root, "missing", &42, &mut context), Ok(false));
	assert_eq!(repl::reset(&mut root, "missing", &mut context), Ok(false));
	assert_eq!(repl::invoke(&mut root, "missing", "", &mut context), Ok(false));
	assert_eq!(repl::set(&mut root, "foo", "42", &mut context), Ok(false));
	assert_eq!(repl::set_value(&mut root, "foo", &42, &mut context), Ok(false));
	assert_eq!(repl::invoke(&mut root, "foo", "", &mut context), Ok(false));
	assert!(context.is_empty());

	let mut empty = Visit(|_| {});
	assert_eq!(repl::poke(&mut empty, "", None, &mut context), Ok(true));
	assert!(context.is_empty());
}

#[test]
fn traversal_combines_lists_and_dotted_names() {
	let mut flat = 1;
	let mut nested = 2;
	let mut deep = 3;
	let mut child = Visit(|f| f(&mut Property("bar.baz", &mut nested, &0)));
	let mut dotted_child = Visit(|f| f(&mut Property("baz", &mut deep, &0)));
	let mut skipped = Visit(|_| panic!("unrelated list was visited"));
	let mut root = Visit(|f| {
		f(&mut Property("foo.bar.baz", &mut flat, &0));
		f(&mut List("foo", &mut child));
		f(&mut List("foo.bar", &mut dotted_child));
		f(&mut Action("foo.bar.baz", |_, _| panic!("listed action was invoked")));
		f(&mut List("fooz", &mut skipped));
	});
	let mut values = String::new();
	assert!(repl::find(&mut root, "foo.bar.baz", |node| {
		match node.as_node() {
			Node::Prop(prop) => values.push_str(&prop.get_value().to_string()),
			Node::Action(_) => values.push('a'),
			Node::List(_) => panic!("unexpected list match"),
		}
	}));
	assert_eq!(values, "123a");
	assert!(!repl::find(&mut root, "foo.bar.ba", |_| panic!("partial name matched")));

	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "foo.bar", Some("ignored"), &mut context), Ok(true));
	assert_eq!(context, concat!(
		"foo.bar.baz is \"1\"\n",
		"foo.bar.baz is \"2\"\n",
		"foo.bar.baz is \"3\"\n",
		"foo.bar.baz\n",
	));
}

#[test]
fn listing_preserves_order_without_invoking_actions() {
	let mut root = root();
	let mut context = String::new();
	repl::print(&mut root, "foo", &mut context);
	assert_eq!(context, concat!(
		"foo.before is \"1\"\n",
		"foo…\n",
		"foo.int is \"13\"\n",
		"foo.float is \"-0.1\"\n",
		"foo.string is \"groot\"\n",
		"foo.action\n",
		"foo.after is \"2\"\n",
	));
	let expected = context.replace("foo…\n", "");
	context.clear();
	assert_eq!(repl::poke(&mut root, "foo", None, &mut context), Ok(true));
	assert_eq!(context, expected);
	context.clear();
	assert_eq!(repl::poke(&mut root, "", None, &mut context), Ok(true));
	assert_eq!(context, "foo.before is \"1\"\nfoo…\nfoo.after is \"2\"\n");
}

#[test]
fn namespaces_work_without_list_nodes() {
	let mut value = 9;
	let mut other = 8;
	let mut root = Visit(|f| {
		f(&mut Property("foo.value", &mut value, &0));
		f(&mut Property("foo.inner.other", &mut other, &0));
		f(&mut Action("foo.command", |_, _| panic!("listed action was invoked")));
	});
	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "foo.inner", None, &mut context), Ok(true));
	assert_eq!(context, "foo.inner.other is \"8\"\n");
	assert_eq!(repl::set(&mut root, "foo.value", "13", &mut context), Ok(true));
	assert_eq!(repl::get_value::<i32>(&mut root, "foo.value"), Some(13));
	assert_eq!(repl::reset(&mut root, "foo", &mut context), Ok(true));
	context.clear();
	repl::print(&mut root, "foo", &mut context);
	assert_eq!(context, "foo.value is \"0\"\nfoo.inner.other is \"0\"\nfoo.command\n");
	context.clear();
	assert_eq!(repl::reset(&mut root, "missing", &mut context), Ok(false));
	assert!(context.is_empty());
}

#[test]
fn change_callbacks_observe_values_in_visitor_order() {
	for args in [None, Some(""), Some("new")] {
		let mut value = String::from("old");
		let mut observed = alloc::vec::Vec::new();
		let mut action_args = alloc::vec::Vec::new();
		let mut root = Visit(|f| {
			f(&mut OnChange("value", |_, _| { observed.push(value.clone()); Ok(()) }));
			f(&mut Property("value", &mut value, &String::new()));
			f(&mut OnChange("value", |_, _| { observed.push(value.clone()); Ok(()) }));
			f(&mut Action("value", |args, _| { action_args.push(args.to_owned()); Ok(()) }));
		});
		assert_eq!(repl::poke(&mut root, "value", args, &mut NullContext), Ok(true));
		assert_eq!(action_args, [args.unwrap_or("")]);
		if let Some(args) = args {
			assert_eq!(observed, ["old", args]);
		}
		else {
			assert!(observed.is_empty());
		}
	}
}

#[test]
fn resets_apply_to_descendants_and_run_only_change_callbacks() {
	for (all, fail) in [(false, false), (false, true), (true, false), (true, true)] {
		let mut flat = 9;
		let mut nested = 8;
		let mut outside = 7;
		let mut observed = alloc::vec::Vec::new();
		let mut child = Visit(|f| f(&mut Property("value", &mut nested, &0)));
		let mut root = Visit(|f| {
			f(&mut OnChange("foo.value", |args, context| {
				assert!(args.is_empty());
				observed.push(flat);
				if fail {
					context.write_error("callback failed");
					Err(ActionError)
				}
				else {
					Ok(())
				}
			}));
			f(&mut Property("foo.value", &mut flat, &0));
			f(&mut List("foo", &mut child));
			f(&mut OnChange("foo.value", |_, context| {
				observed.push(flat);
				writeln!(context, "reset to {flat}");
				Ok(())
			}));
			f(&mut Action("foo.command", |_, _| panic!("ordinary action was invoked")));
			f(&mut Property("foobar.value", &mut outside, &0));
		});
		let mut context = String::new();
		let result = if all {
			repl::reset_all(&mut root, &mut context).map(|()| true)
		}
		else {
			repl::reset(&mut root, "foo", &mut context)
		};
		assert_eq!(result, if fail { Err(ActionError) } else { Ok(true) });
		assert_eq!(context, if fail { "error: callback failed\nreset to 0\n" } else { "reset to 0\n" });
		assert_eq!((flat, nested, outside), (0, 0, if all { 0 } else { 7 }));
		assert_eq!(observed, [9, 0]);
	}
}

enum_string! {
	#[derive(Clone, Debug, Eq, PartialEq)]
	enum Mode { First, Second }
}

#[test]
fn writable_properties_support_a_shared_lifecycle() {
	let mut value = Mode::First;
	let mut borrowed = Property("borrowed", &mut value, &Mode::First);
	let mut owned = OwnedProp(String::from("owned"), Mode::First, Mode::First);
	for prop in [&mut borrowed as &mut dyn IProperty, &mut owned] {
		assert_eq!(prop.state(), PropState::Default);
		assert_eq!(prop.set("Second", &mut NullContext), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<Mode>(), Some(&Mode::Second));
		assert_eq!(prop.state(), PropState::UserSet);
		assert_eq!(prop.set("Missing", &mut NullContext), Err(ActionError));
		assert_eq!(prop.set_value(&42, &mut NullContext), Err(ActionError));
		assert_eq!(prop.get_value().downcast_ref::<Mode>(), Some(&Mode::Second));
		assert_eq!(prop.set_value(&Mode::First, &mut NullContext), Ok(()));
		assert_eq!(prop.state(), PropState::Default);
		assert_eq!(prop.set_value(&Mode::Second, &mut NullContext), Ok(()));
		assert_eq!(prop.reset(&mut NullContext), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<Mode>(), Some(&Mode::First));
	}
}

#[test]
fn read_only_properties_reject_changes() {
	#[derive(PartialEq)]
	struct Readout(u32);
	impl fmt::Display for Readout {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{} readings", self.0) }
	}
	let value = Readout(42);
	let default = Readout(0);
	let mut prop = ReadOnlyProp("readout", &value, &default);
	let mut context = String::new();
	assert_eq!(prop.get_value().to_string(), "42 readings");
	assert_eq!(prop.set("0", &mut context), Err(ActionError));
	assert_eq!(prop.set_value(&default, &mut context), Err(ActionError));
	assert_eq!(context, "error: cannot set read-only property\nerror: cannot set read-only property\n");
	context.clear();
	assert_eq!(prop.reset(&mut context), Err(ActionError));
	assert_eq!(context, "error: cannot reset read-only property\n");
	assert_eq!(prop.get_value().downcast_ref::<Readout>().unwrap().0, 42);
	assert_eq!(prop.state(), PropState::UserSet);
}

#[test]
fn clamped_properties_enforce_bounds_and_reject_invalid_values() {
	for (min, max, input, expected) in [
		(Some(0), Some(10), -1, 0),
		(Some(0), Some(10), 11, 10),
		(Some(0), Some(10), 7, 7),
		(None, Some(10), -10, -10),
		(Some(0), None, -1, 0),
		(None, None, 20, 20),
	] {
		let mut value = 5;
		let mut prop = ClampedProp("value", &mut value, &5, min.as_ref(), max.as_ref());
		assert_eq!(prop.set(&input.to_string(), &mut NullContext), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
		assert_eq!(prop.reset(&mut NullContext), Ok(()));
		assert_eq!(prop.set_value(&input, &mut NullContext), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
		assert_eq!(prop.reset(&mut NullContext), Ok(()));
		assert_eq!(value, 5);
	}
	let mut value = 0.5f32;
	let mut prop = ClampedProp("value", &mut value, &0.5, Some(&0.0), Some(&1.0));
	assert_eq!(prop.set("NaN", &mut NullContext), Err(ActionError));
	assert_eq!(prop.set_value(&f32::NAN, &mut NullContext), Err(ActionError));
	assert_eq!(value, 0.5);
	let mut value = 5;
	let mut prop = ClampedProp("value", &mut value, &5, Some(&10), Some(&0));
	assert_eq!(prop.set("1", &mut NullContext), Err(ActionError));
	assert_eq!(value, 5);
}

#[test]
fn operations_report_failure_and_continue_visiting_matches() {
	for operation in 0..4 {
		let mut value = 7;
		let mut calls = String::new();
		let mut root = Visit(|f| {
			f(&mut Property("value", &mut value, &0));
			f(&mut Action("value", |_, context| {
				calls.push('a');
				if operation == 0 {
					context.write_error("action failed");
					Err(ActionError)
				}
				else {
					Ok(())
				}
			}));
			f(&mut Action("value", |_, _| { calls.push('b'); Ok(()) }));
		});
		let mut context = String::new();
		let result = match operation {
			0 => repl::invoke(&mut root, "value", "invalid", &mut context),
			1 => repl::set(&mut root, "value", "invalid", &mut context),
			2 => repl::set_value(&mut root, "value", &true, &mut context),
			_ => repl::poke(&mut root, "value", Some("invalid"), &mut context),
		};
		assert_eq!(result, Err(ActionError));
		assert_eq!(value, 7);
		assert_eq!(calls, "ab");
		assert!(context.starts_with("error: "));
	}
}

#[test]
fn actions_can_access_context_data_and_write_output() {
	struct Context { output: String, calls: usize }
	impl IContext for Context {
		fn write_str(&mut self, s: &str) { self.output.push_str(s); }
		fn request_any_ref(&self, ty: any::TypeId) -> Option<&dyn any::Any> {
			if ty == any::TypeId::of::<usize>() { Some(&self.calls) } else { None }
		}
		fn request_any_mut(&mut self, ty: any::TypeId) -> Option<&mut dyn any::Any> {
			if ty == any::TypeId::of::<usize>() { Some(&mut self.calls) } else { None }
		}
	}
	let mut root = Visit(|f| f(&mut Action("run", |args, context| {
		let calls = *context.request_ref::<usize>().ok_or(ActionError)?;
		*context.request_mut::<usize>().ok_or(ActionError)? = calls + 1;
		write!(context, "{args}");
		context.write_char('é');
		writeln!(context);
		Ok(())
	})));
	let mut context = Context { output: String::new(), calls: 0 };
	assert_eq!(repl::invoke(&mut root, "run", "caf", &mut context), Ok(true));
	assert_eq!(context.calls, 1);
	assert_eq!(context.output, "café\n");
	assert_eq!(repl::invoke(&mut root, "run", "", &mut NullContext), Err(ActionError));
}

#[cfg(feature = "std")]
#[test]
fn output_errors_do_not_abort_actions() {
	let mut called = false;
	let mut root = Visit(|f| f(&mut Action("run", |_, context| {
		writeln!(context, "Hello!");
		called = true;
		Ok(())
	})));
	assert_eq!(repl::invoke(&mut root, "run", "", &mut IoWriter(&mut [][..])), Ok(true));
	assert!(called);
}
