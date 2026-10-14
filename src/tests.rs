use crate::*;

#[test]
fn poke_only_invokes_change_callbacks_when_arguments_are_present() {
	for args in [None, Some(""), Some("new")] {
		let mut value = String::from("old");
		let mut observed = alloc::vec::Vec::new();
		let mut action_args = alloc::vec::Vec::new();
		let mut root = Visit(|f| {
			f(&mut OnChange("value", |_, _| {
				observed.push(value.clone());
				Ok(())
			}));
			f(&mut Property("value", &mut value, &String::new()));
			f(&mut OnChange("value", |_, _| {
				observed.push(value.clone());
				Ok(())
			}));
			f(&mut Action("value", |args, _| {
				action_args.push(args.to_owned());
				Ok(())
			}));
		});
		let mut context = String::new();
		assert_eq!(repl::poke(&mut root, "value", args, &mut context), Ok(()));
		assert_eq!(value, args.unwrap_or("old"));
		assert_eq!(context, format!("value is {value:?}\n"));
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
fn resets_only_invoke_change_callbacks_in_visitor_order() {
	struct Config {
		value: i32,
		observed: alloc::vec::Vec<i32>,
	}
	impl IVisit for Config {
		fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
			f(&mut OnChange("value", |args, context| {
				assert!(args.is_empty());
				assert!(context.request_ref::<i32>().is_none());
				self.observed.push(self.value);
				Err(ActionError)
			}));
			f(&mut Property("value", &mut self.value, &0));
			f(&mut OnChange("value", |_, _| {
				self.observed.push(self.value);
				Ok(())
			}));
			f(&mut Action("value", |_, _| panic!("ordinary same-name action invoked")));
			f(&mut Action("unrelated", |_, _| panic!("unrelated action invoked")));
		}
	}
	let mut config = Config { value: 42, observed: alloc::vec::Vec::new() };
	assert!(repl::reset(&mut config, "value"));
	assert_eq!(config.value, 0);
	assert_eq!(config.observed, [42, 0]);
	config.value = 7;
	config.observed.clear();
	let mut root = Visit(|f| f(&mut List("nested", &mut config)));
	repl::reset_all(&mut root);
	assert_eq!(config.value, 0);
	assert_eq!(config.observed, [7, 0]);
}

enum_string! {
	#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
	enum ValueWithoutDefault {
		First,
		Second,
		Third,
	}
}

#[test]
fn values_and_writable_properties_without_default() {
	use ValueWithoutDefault::{First, Second, Third};

	let value: &dyn IValue = &Second;
	assert!(value.is::<ValueWithoutDefault>());
	assert_eq!(value.downcast_ref::<ValueWithoutDefault>(), Some(&Second));
	assert_eq!(value.to_string(), "Second");
	assert_eq!(format!("{value:?}"), "Second");
	#[cfg(feature = "type_name")]
	assert_eq!(value.type_name(), any::type_name::<ValueWithoutDefault>());

	let mut borrowed_value = First;
	let mut clamped_value = First;
	let mut borrowed = Property("borrowed", &mut borrowed_value, &First);
	let mut clamped = ClampedProp("clamped", &mut clamped_value, &First, Some(&First), Some(&Second));
	let mut owned = OwnedProp(String::from("owned"), First, First);
	let mut context = NullContext;
	let properties: [&mut dyn INode; 3] = [&mut borrowed, &mut clamped, &mut owned];
	for node in properties {
		let Node::Prop(prop) = node.as_node() else {
			panic!("expected a property node");
		};
		assert_eq!(prop.state(), PropState::Default);
		assert_eq!(prop.default_value().downcast_ref::<ValueWithoutDefault>(), Some(&First));
		assert_eq!(prop.set("Second", &mut context), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&Second));
		assert_eq!(prop.state(), PropState::UserSet);
		assert_eq!(prop.set("Missing", &mut context), Err(ActionError));
		assert_eq!(prop.set_value(&42, &mut context), Err(ActionError));
		assert_eq!(prop.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&Second));
		prop.reset();
		assert_eq!(prop.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&First));
		assert_eq!(prop.state(), PropState::Default);
		assert_eq!(prop.set_value(&Second, &mut context), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&Second));
		prop.reset();
		assert_eq!(prop.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&First));
	}
	assert_eq!(clamped.set("Third", &mut context), Ok(()));
	assert_eq!(clamped.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&Second));
	clamped.reset();
	assert_eq!(clamped.set_value(&Third, &mut context), Ok(()));
	assert_eq!(clamped.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&Second));
}

#[test]
fn read_only_property_supports_values_without_clone_or_parsing() {
	#[derive(PartialEq)]
	struct Readout(u32);
	impl fmt::Display for Readout {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(f, "{} readings", self.0)
		}
	}
	let value = Readout(42);
	let default = Readout(0);
	let mut prop = ReadOnlyProp("readout", &value, &default);
	assert!(prop.get_value().is::<Readout>());
	assert_eq!(prop.get_value().downcast_ref::<Readout>().unwrap().0, 42);
	assert_eq!(prop.default_value().downcast_ref::<Readout>().unwrap().0, 0);
	assert_eq!(prop.get_value().to_string(), "42 readings");
	assert_eq!(prop.state(), PropState::UserSet);
	assert_eq!(prop.set_value(&value, &mut NullContext), Err(ActionError));
	prop.reset();
	assert_eq!(prop.get_value().downcast_ref::<Readout>().unwrap().0, 42);
}

#[test]
fn property_getters_can_supply_setter_values() {
	let source = ReadOnlyProp("source", &42, &0);
	let mut destination = OwnedProp(String::from("destination"), 0, 0);
	assert_eq!(destination.set_value(source.get_value(), &mut NullContext), Ok(()));
	assert_eq!(destination.variable, 42);
	assert_eq!(destination.set_value(source.default_value(), &mut NullContext), Ok(()));
	assert_eq!(destination.variable, 0);
}

#[test]
fn read_only_property_without_default() {
	use ValueWithoutDefault::{First, Second};

	let mut prop = ReadOnlyProp("read_only", &Second, &First);
	let mut context = String::new();
	let Node::Prop(prop) = prop.as_node() else {
		panic!("expected a property node");
	};
	assert_eq!(prop.default_value().downcast_ref::<ValueWithoutDefault>(), Some(&First));
	assert_eq!(prop.state(), PropState::UserSet);
	assert_eq!(prop.set("First", &mut context), Err(ActionError));
	assert_eq!(prop.set_value(&First, &mut context), Err(ActionError));
	assert_eq!(context, "error: cannot set read-only property\nerror: cannot set read-only property\n");
	prop.reset();
	assert_eq!(prop.get_value().downcast_ref::<ValueWithoutDefault>(), Some(&Second));
	assert_eq!(prop.state(), PropState::UserSet);
}

struct Foo {
	int: i32,
	float: f32,
	string: String,
}
impl Foo {
	fn action(&mut self, arg: &str, context: &mut dyn IContext) -> ActionResult {
		writeln!(context, "I am {} and {}", self.string, arg)?;
		Ok(())
	}
}
impl IVisit for Foo {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		f(&mut Property::new("int", &mut self.int, &42));
		f(&mut Property::new("float", &mut self.float, &1.2f32));
		f(&mut Property::new("string", &mut self.string, &String::new()));
		f(&mut Action::new("action", |args, context| self.action(args, context)));
	}
}
struct Root {
	before: i32,
	foo: Foo,
	after: i32,
}
impl IVisit for Root {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn INode)) {
		f(&mut Property::new("foo.before", &mut self.before, &1));
		f(&mut List::new("foo", &mut self.foo));
		f(&mut Property::new("foo.after", &mut self.after, &2));
	}
}
fn root() -> Root {
	Root {
		before: 1,
		foo: Foo {
			int: 13,
			float: -0.1f32,
			string: String::from("groot"),
		},
		after: 2,
	}
}
#[test]
fn main() {
	let mut root = root();
	let mut context = NullContext;
	assert_eq!(repl::set(&mut root, "foo.float", "-1", &mut context), Ok(()));
	assert_eq!(root.foo.float, -1.0f32);
	assert_eq!(repl::set(&mut root, "foo.before", "11", &mut context), Ok(()));
	assert_eq!(repl::set(&mut root, "foo.after", "22", &mut context), Ok(()));
	assert_eq!(repl::set(&mut root, "foo.int", "parse error", &mut context), Err(ActionError));
	assert_eq!(repl::set(&mut root, "foo.list.bar", "name error", &mut context), Err(ActionError));
	assert_eq!(repl::set(&mut root, "foo.int.bar", "list error", &mut context), Err(ActionError));
	assert_eq!(repl::set(&mut root, "foo.action.bar", "list error", &mut context), Err(ActionError));
	assert_eq!(repl::set(&mut root, "foo.action", "args", &mut context), Ok(()));

	assert_eq!(repl::set_value(&mut root, "foo.int", &42, &mut context), Ok(()));
	assert_eq!(repl::get_value::<i32>(&mut root, "foo.int"), Some(42));

	assert_eq!(repl::set(&mut root, "foo.string", "any", &mut context), Ok(()));
	assert_eq!(repl::get_value::<String>(&mut root, "foo.string"), Some(String::from("any")));
}

#[test]
fn find_dotted_names_without_list_parents() {
	// The prefix may be absent, a property, or an action.
	for parent in 0..3 {
		let mut prefix = 7;
		let mut value = 42;
		let mut action_calls = 0;
		let mut root = Visit(|f| {
			match parent {
				1 => f(&mut Property("foo", &mut prefix, &0)),
				2 => f(&mut Action("foo", |_, _| panic!("prefix action was invoked"))),
				_ => {},
			}
			f(&mut Property("foo.bar", &mut value, &0));
			f(&mut Action("foo.run", |_, _| { action_calls += 1; Ok(()) }));
		});
		let mut names = String::new();
		assert_eq!(repl::find(&mut root, "foo", |node| names.push_str(node.name())), parent != 0);
		assert_eq!(names, if parent == 0 { "" } else { "foo" });
		names.clear();
		assert!(repl::find(&mut root, "foo.bar", |node| names.push_str(node.name())));
		assert_eq!(names, "foo.bar");
		for path in ["foo.ba", "foo.bar.baz", "foo.missing", "fo"] {
			assert!(!repl::find(&mut root, path, |_| panic!("unexpected match")));
		}
		assert_eq!(repl::get_value::<i32>(&mut root, "foo.bar"), Some(42));
		assert_eq!(repl::set(&mut root, "foo.bar", "13", &mut NullContext), Ok(()));
		assert_eq!(repl::invoke(&mut root, "foo.run", "", &mut NullContext), Ok(()));
		assert_eq!(value, 13);
		assert_eq!(prefix, 7);
		assert_eq!(action_calls, 1);
	}
}

#[test]
fn find_matches_all_combinations_of_lists_and_dotted_names() {
	let mut flat_value = 1;
	let mut nested_value = 2;
	let mut dotted_list_value = 3;
	let mut child = Visit(|f| f(&mut Property("bar.baz", &mut nested_value, &0)));
	let mut dotted_child = Visit(|f| f(&mut Property("baz", &mut dotted_list_value, &0)));
	let mut skipped = Visit(|_| panic!("unrelated list was visited"));
	let mut root = Visit(|f| {
		f(&mut Property("foo.bar.baz", &mut flat_value, &0));
		f(&mut List("foo", &mut child));
		f(&mut List("foo.bar", &mut dotted_child));
		f(&mut Action("foo.bar.baz", |_, _| panic!("action was invoked")));
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
}

#[test]
fn find_yields_duplicate_list_paths_and_leaves_recursion_to_the_caller() {
	let mut value = 9;
	let mut visits = 0;
	let mut extra = 8;
	let mut leaf = Visit(|f| {
		visits += 1;
		f(&mut Property("value", &mut value, &0));
	});
	let mut root = Visit(|f| {
		f(&mut List("foo.bar", &mut leaf));
		let mut foo = Visit(|f| f(&mut List("bar", &mut leaf)));
		f(&mut List("foo", &mut foo));
		f(&mut Property("foo.bar.extra", &mut extra, &0));
	});
	let mut names = alloc::vec::Vec::new();
	assert!(repl::find(&mut root, "foo.bar", |node| names.push(node.name().to_owned())));
	assert_eq!(names, ["foo.bar", "bar"]);
	let mut values = alloc::vec::Vec::new();
	assert!(repl::find(&mut root, "foo.bar", |node| {
		let Node::List(list) = node.as_node() else { panic!("expected list") };
		list.visitor().visit(&mut |child| values.push(child.name().to_owned()));
	}));
	assert_eq!(values, ["value", "value"]);
	assert_eq!(visits, 2, "only the caller's explicit recursion visits matched lists");
}

#[test]
fn reset_includes_dotted_siblings_and_lists_once_in_visitor_order() {
	let mut flat = 9;
	let mut nested = 8;
	let mut deep = 7;
	let mut outside = 6;
	let mut observed = alloc::vec::Vec::new();
	let mut child_calls = 0;
	let mut child = Visit(|f| {
		f(&mut Property("value", &mut nested, &0));
		f(&mut OnChange("value", |_, _| { child_calls += 1; Ok(()) }));
		f(&mut Action("value", |_, _| panic!("ordinary action invoked")));
	});
	let mut dotted_child = Visit(|f| f(&mut Property("value", &mut deep, &0)));
	let mut skipped = Visit(|_| panic!("unrelated list visited"));
	let mut root = Visit(|f| {
		f(&mut OnChange("foo.value", |_, _| { observed.push(flat); Ok(()) }));
		f(&mut Property("foo.value", &mut flat, &0));
		f(&mut List("foo", &mut child));
		f(&mut List("foo.more", &mut dotted_child));
		f(&mut OnChange("foo.value", |_, _| { observed.push(flat); Err(ActionError) }));
		f(&mut Action("foo.command", |_, _| panic!("command invoked")));
		f(&mut Property("foobar.value", &mut outside, &0));
		f(&mut List("foobar", &mut skipped));
	});
	assert!(!repl::reset(&mut root, "missing"));
	assert!(repl::reset(&mut root, "foo"));
	assert_eq!((flat, nested, deep, outside), (0, 0, 0, 6));
	assert_eq!(observed, [9, 0]);
	assert_eq!(child_calls, 1);
}

#[test]
fn reset_and_print_support_namespaces_without_real_lists() {
	let mut value = 9;
	let mut other = 8;
	let mut root = Visit(|f| {
		f(&mut Property("foo.value", &mut value, &0));
		f(&mut Property("foo.inner.other", &mut other, &0));
		f(&mut Action("foo.command", |_, _| panic!("listed action invoked")));
	});
	assert!(repl::reset(&mut root, "foo"));
	let mut context = String::new();
	repl::print(&mut root, "foo", &mut context).unwrap();
	assert_eq!(context, "foo.value is \"0\"\nfoo.inner.other is \"0\"\nfoo.command\n");
	context.clear();
	repl::print(&mut root, "missing", &mut context).unwrap();
	assert_eq!(context, "error: path \"missing\" does not exist\n");
}

#[test]
fn print_includes_real_lists_and_dotted_siblings_without_invoking_actions() {
	let mut root = root();
	let mut context = String::new();
	repl::print(&mut root, "foo", &mut context).unwrap();
	assert_eq!(context, concat!(
		"foo.before is \"1\"\n",
		"foo…\n",
		"foo.int is \"13\"\n",
		"foo.float is \"-0.1\"\n",
		"foo.string is \"groot\"\n",
		"foo.action\n",
		"foo.after is \"2\"\n",
	));
}

#[test]
fn print_propagates_write_errors_and_continues_visiting() {
	struct FailOnceContext {
		writes: usize,
		output: String,
	}
	impl fmt::Write for FailOnceContext {
		fn write_str(&mut self, s: &str) -> fmt::Result {
			self.writes += 1;
			if self.writes == 1 {
				Err(fmt::Error)
			}
			else {
				self.output.push_str(s);
				Ok(())
			}
		}
	}
	impl IContext for FailOnceContext {}

	for path in ["", "foo", "foo.int", "missing"] {
		let mut root = root();
		let mut context = FailOnceContext { writes: 0, output: String::new() };
		assert_eq!(repl::print(&mut root, path, &mut context), Err(fmt::Error));
		if path == "" || path == "foo" {
			assert!(context.output.ends_with("foo.after is \"2\"\n"));
		}
	}
}

#[test]
fn poke_lists_dotted_siblings_and_real_list_children() {
	let mut root = root();
	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "foo", None, &mut context), Ok(()));
	assert_eq!(context, concat!(
		"foo.before is \"1\"\n",
		"foo.int is \"13\"\n",
		"foo.float is \"-0.1\"\n",
		"foo.string is \"groot\"\n",
		"foo.action\n",
		"foo.after is \"2\"\n",
	));
	assert_eq!(root.foo.string, "groot");
}

#[test]
fn poke_lists_fake_scopes_and_deeper_paths() {
	let mut value = 42;
	let mut other = true;
	let mut unrelated = 7;
	let mut root = Visit(|f| {
		f(&mut Property("scope.value", &mut value, &0));
		f(&mut Property("scope.inner.other", &mut other, &false));
		f(&mut Property("scope2.value", &mut unrelated, &0));
	});
	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "scope", None, &mut context), Ok(()));
	assert_eq!(context, "scope.value is \"42\"\nscope.inner.other is \"true\"\n");
	context.clear();
	assert_eq!(repl::poke(&mut root, "scope.inner", None, &mut context), Ok(()));
	assert_eq!(context, "scope.inner.other is \"true\"\n");
	context.clear();
	assert_eq!(repl::poke(&mut root, "missing", None, &mut context), Err(ActionError));
	assert_eq!(context, "error: path \"missing\" does not exist\n");
	context.clear();
	assert_eq!(repl::poke(&mut root, "scope.value", Some("13"), &mut context), Ok(()));
	assert_eq!(context, "scope.value is \"13\"\n");
}

#[test]
fn poke_lists_nested_real_lists_without_invoking_actions() {
	let mut value = 42;
	let mut calls = 0;
	let mut child = Visit(|f| {
		f(&mut Property("value", &mut value, &0));
		f(&mut Action("action", |_, _| { calls += 1; Ok(()) }));
	});
	let mut inner = Visit(|f| f(&mut List("inner", &mut child)));
	let mut root = Visit(|f| f(&mut List("scope", &mut inner)));
	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "scope", Some("args"), &mut context), Ok(()));
	assert_eq!(context, "scope.inner…\nscope.inner.value is \"42\"\nscope.inner.action\n");
	context.clear();
	assert_eq!(repl::poke(&mut root, "scope.inner.action", Some("args"), &mut context), Ok(()));
	assert!(context.is_empty());
	assert_eq!(calls, 1);
}

#[test]
fn poke_skips_unrelated_list_visitors() {
	let mut skipped = Visit(|_| panic!("unrelated list was visited"));
	let mut skipped_inside_scope = Visit(|_| panic!("unrelated child list was visited"));
	let mut value = 42;
	let mut extra = 7;
	let mut child = Visit(|f| f(&mut Property("value", &mut value, &0)));
	let mut scope = Visit(|f| {
		f(&mut List("other", &mut skipped_inside_scope));
		f(&mut List("inner", &mut child));
	});
	let mut root = Visit(|f| {
		f(&mut List("unrelated", &mut skipped));
		f(&mut List("scope2", &mut skipped));
		f(&mut List("scope", &mut scope));
		f(&mut Property("scope.inner.extra", &mut extra, &0));
	});
	let mut context = String::new();
	assert_eq!(repl::poke(&mut root, "scope.inner", None, &mut context), Ok(()));
	assert_eq!(context, "scope.inner.value is \"42\"\nscope.inner.extra is \"7\"\n");
	context.clear();
	assert_eq!(repl::poke(&mut root, "scope.inner.value", Some("13"), &mut context), Ok(()));
	assert_eq!(context, "scope.inner.value is \"13\"\n");
	context.clear();
	assert_eq!(repl::poke(&mut root, "scope.missing", None, &mut context), Err(ActionError));
	assert_eq!(context, "error: path \"scope.missing\" does not exist\n");
}

#[test]
fn clamped_property_clamps_to_both_bounds() {
	let mut value = 5;
	let mut prop = ClampedProp("value", &mut value, &5, Some(&0), Some(&10));
	let mut context = String::new();
	for (val, expected) in [(-1, 0), (11, 10), (0, 0), (10, 10), (7, 7)] {
		prop.reset();
		assert_eq!(prop.set(&val.to_string(), &mut context), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
		prop.reset();
		assert_eq!(prop.set_value(&val, &mut context), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
	}
	assert!(context.is_empty());
}

#[test]
fn clamped_property_clamps_one_sided_bounds() {
	let mut value = 5;
	let mut context = NullContext;
	for (min, max, val, expected) in [
		(None, Some(10), -10, -10),
		(None, Some(10), 11, 10),
		(Some(0), None, 20, 20),
		(Some(0), None, -1, 0),
		(None, None, -20, -20),
		(None, None, 20, 20),
	] {
		let mut prop = ClampedProp("value", &mut value, &5, min.as_ref(), max.as_ref());
		prop.reset();
		assert_eq!(prop.set(&val.to_string(), &mut context), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
		prop.reset();
		assert_eq!(prop.set_value(&val, &mut context), Ok(()));
		assert_eq!(prop.get_value().downcast_ref::<i32>(), Some(&expected));
	}
}

#[test]
fn clamped_property_rejects_nan_and_invalid_assignments() {
	let mut context = String::new();
	let mut value = 0.5f32;
	let mut prop = ClampedProp("value", &mut value, &0.5, Some(&0.0), Some(&1.0));
	assert_eq!(prop.set("NaN", &mut context), Err(ActionError));
	assert_eq!(prop.set_value(&f32::NAN, &mut context), Err(ActionError));
	assert_eq!(prop.set("invalid", &mut context), Err(ActionError));
	assert_eq!(prop.set_value(&42i32, &mut context), Err(ActionError));
	assert_eq!(value, 0.5);
	assert_eq!(context.lines().count(), 4);
	assert!(context.ends_with('\n'));

	let mut value = 5;
	let mut prop = ClampedProp("value", &mut value, &5, Some(&10), Some(&0));
	assert_eq!(prop.set("-1", &mut context), Err(ActionError));
	assert_eq!(prop.set_value(&11, &mut context), Err(ActionError));
	assert_eq!(value, 5);
	assert_eq!(context.lines().count(), 6);
	assert!(context.ends_with("error: cannot clamp value to bounds\nerror: cannot clamp value to bounds\n"));
}

#[test]
fn actions_propagate_context_write_errors() {
	struct FailingContext;
	impl fmt::Write for FailingContext {
		fn write_str(&mut self, _s: &str) -> fmt::Result {
			Err(fmt::Error)
		}
	}
	impl IContext for FailingContext {}

	for operation in 0..4 {
		let mut calls = 0;
		let mut root = Visit(|f| {
			f(&mut Action("action", |_, context| {
				writeln!(context, "Hello!")?;
				calls += 1;
				Ok(())
			}));
		});
		for failed in [false, true] {
			let mut working = String::new();
			let mut failing = FailingContext;
			let context: &mut dyn IContext = if failed { &mut failing } else { &mut working };
			let result = match operation {
				0 => repl::invoke(&mut root, "action", "", context),
				1 => repl::set(&mut root, "action", "", context),
				2 => repl::set_value(&mut root, "action", &42, context),
				_ => repl::poke(&mut root, "action", None, context),
			};
			assert_eq!(result, if failed { Err(ActionError) } else { Ok(()) });
			assert_eq!(working, if failed { "" } else { "Hello!\n" });
		}
		assert_eq!(calls, 1);
	}
}

#[test]
fn actions_propagate_results_and_visit_all_matches() {
	let mut context = String::new();
	let mut action = Action("action", |args, context| {
		if args == "ok" {
			Ok(())
		}
		else {
			context.write_error(&format_args!("action failed: {args}"))?;
			Err(ActionError)
		}
	});
	assert_eq!(action.invoke("ok", &mut context), Ok(()));
	assert_eq!(action.invoke("bad", &mut context), Err(ActionError));
	assert_eq!(context, "error: action failed: bad\n");

	for failed in [None, Some(0), Some(1)] {
		for operation in 0..4 {
			let mut calls = String::new();
			let mut root = Visit(|f| {
				for i in 0..2 {
					f(&mut Action("action", |args, context| {
						assert_eq!(args, "42");
						calls.push(char::from(b'0' + i));
						if failed == Some(i) {
							context.write_error(&format_args!("action {i} failed"))?;
							Err(ActionError)
						}
						else {
							Ok(())
						}
					}));
				}
			});
			context.clear();
			let result = match operation {
				0 => repl::invoke(&mut root, "action", "42", &mut context),
				1 => repl::set(&mut root, "action", "42", &mut context),
				2 => repl::set_value(&mut root, "action", &42, &mut context),
				_ => repl::poke(&mut root, "action", Some("42"), &mut context),
			};
			assert_eq!(result, if failed.is_some() { Err(ActionError) } else { Ok(()) });
			assert_eq!(calls, "01");
			assert_eq!(context, failed.map_or(String::new(), |i| format!("error: action {i} failed\n")));
		}
	}
}

#[test]
fn failed_property_assignments_are_not_hidden_by_action_aliases() {
	for operation in 0..3 {
		let mut value = 7;
		let mut calls = 0;
		let mut root = Visit(|f| {
			f(&mut Property("value", &mut value, &0));
			f(&mut Action("value", |_, _| { calls += 1; Ok(()) }));
		});
		let mut context = String::new();
		let result = match operation {
			0 => repl::set(&mut root, "value", "invalid", &mut context),
			1 => repl::set_value(&mut root, "value", &true, &mut context),
			_ => repl::poke(&mut root, "value", Some("invalid"), &mut context),
		};
		assert_eq!(result, Err(ActionError));
		assert_eq!(value, 7);
		assert_eq!(calls, 1);
		if operation == 1 {
			assert!(context.starts_with("error: mismatched types"));
			assert!(context.ends_with('\n'));
		}
		else {
			assert_eq!(context, format!("error: {}\n", "invalid".parse::<i32>().unwrap_err()));
		}
	}
}

#[test]
fn repl_rejects_missing_or_inapplicable_nodes() {
	let mut root = root();
	let mut context = String::new();
	assert_eq!(repl::invoke(&mut root, "missing", "", &mut context), Err(ActionError));
	assert_eq!(repl::invoke(&mut root, "foo.int", "", &mut context), Err(ActionError));
	assert_eq!(repl::invoke(&mut root, "foo", "", &mut context), Err(ActionError));
	assert_eq!(repl::set(&mut root, "foo", "42", &mut context), Err(ActionError));
	assert_eq!(repl::set_value(&mut root, "foo", &42, &mut context), Err(ActionError));
	assert_eq!(repl::set_value(&mut root, "missing", &42, &mut context), Err(ActionError));
	assert_eq!(context, "error: path \"missing\" does not exist\n");
}

#[test]
fn property_setters_receive_application_context() {
	struct Context {
		output: String,
		calls: usize,
	}
	impl fmt::Write for Context {
		fn write_str(&mut self, s: &str) -> fmt::Result {
			self.output.push_str(s);
			Ok(())
		}
	}
	impl IContext for Context {
		fn request_any_ref(&self, ty: any::TypeId) -> Option<&dyn any::Any> {
			if ty == any::TypeId::of::<usize>() { Some(&self.calls) } else { None }
		}
		fn request_any_mut(&mut self, ty: any::TypeId) -> Option<&mut dyn any::Any> {
			if ty == any::TypeId::of::<usize>() { Some(&mut self.calls) } else { None }
		}
	}
	struct ContextProp(OwnedProp<i32>);
	impl INode for ContextProp {
		fn name(&self) -> &str { self.0.name() }
		fn as_node(&mut self) -> Node<'_> { Node::Prop(self) }
	}
	impl IProperty for ContextProp {
		fn get_value(&self) -> &dyn IValue { self.0.get_value() }
		fn default_value(&self) -> &dyn IValue { self.0.default_value() }
		fn state(&self) -> PropState { self.0.state() }
		fn reset(&mut self) { self.0.reset(); }
		fn set(&mut self, val: &str, context: &mut dyn IContext) -> ActionResult {
			let calls = context.request_ref::<usize>().copied().ok_or(ActionError)?;
			*context.request_mut::<usize>().ok_or(ActionError)? = calls + 1;
			self.0.set(val, context)?;
			writeln!(context, "updated")?;
			Ok(())
		}
		fn set_value(&mut self, val: &dyn IValue, context: &mut dyn IContext) -> ActionResult {
			let val = val.downcast_ref::<i32>().ok_or(ActionError)?;
			self.set(&val.to_string(), context)
		}
	}
	let mut prop = ContextProp(OwnedProp("value".into(), 0, 0));
	let mut root = Visit(|f| f(&mut prop));
	let mut context = Context { output: String::new(), calls: 0 };
	assert_eq!(repl::set(&mut root, "value", "1", &mut context), Ok(()));
	assert_eq!(repl::set_value(&mut root, "value", &2, &mut context), Ok(()));
	assert_eq!(repl::poke(&mut root, "value", Some("3"), &mut context), Ok(()));
	assert_eq!(repl::set(&mut root, "value", "invalid", &mut context), Err(ActionError));
	assert_eq!(context.calls, 4);
	assert_eq!(context.output, format!("updated\nupdated\nupdated\nvalue is \"3\"\nerror: {}\n", "invalid".parse::<i32>().unwrap_err()));
	assert_eq!(prop.0.variable, 3);
}
