Configuration Variables
=======================

Configure programs through variables and actions.

## Introduction

When an application does a particular job and exits, configuration can be loaded at startup and stored in a dedicated configuration object.

When an application is long-lived this may not suffice, it may want to have:

* Its configuration changed one setting at the time.

* Complex hierarchical organization of the configuration.

* An interface where the state of the configuration can be queried and updated.

* Executable actions instead of just variables.

* Addon support which needs its own configuration mounted under a prefix.

* Support dynamic configuration created at runtime.

## Design

Configuration is the interface where a user meets and interacts with the program state. Users interact through text while programs have their preferred data types.

The goal of a configuration manager is to facilitate this interaction. This library's scope is more narrow in providing just the means to identify configurable elements and interact with them through text.

A configurable element is called a _node_. Every _node_ has some metadata such as a _name_ and a _description_. There are three types of nodes: _properties_, _lists_ and _actions_.

* A _property_ stores a variable state. It has a _default value_ and an optional _change_ callback.

* A _list_ defines a hierarchy. It contains _child nodes_. Names in a hierarchy are separated by a dot, eg: `list.foo`.

* An _action_ is node that can be invoked with an argument string. It can execute arbitrary code in its context.

The _nodes_ are ephemeral, this means that they don't store their metadata next to and interspersed with the program state. This avoids requiring long-lived borrows and has other performance benefits.

While generic implementations are provided, each node type is a trait and can have custom implementations.

This library aims to be lightweight, avoid unnecessary memory allocations and unavoidable debug strings. The ephemeral nature of the metadata allows obfuscation.

For technical documentation see the docs below.

## Future work

Implement autocomplete for identifiers and suggestions for property values and actions.

Implement helpers for enums and enum flags support.

## Usage

This crate can be found on [crates.io](https://crates.io/crates/cvar).

Documentation can be found [online](https://casualx.github.io/docs/cvar-rs/0.1.0/cvar).

In your `Cargo.toml`:

```
[dependencies]
cvar = "0.1"
```

## License

MIT, see LICENSE.txt
