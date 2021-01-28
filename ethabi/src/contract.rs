// Copyright 2015-2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::operation::Operation;
use crate::{errors, Constructor, Error, Event, Function};
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_map::Values;
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use core::fmt;
use core::iter::Flatten;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
#[cfg(feature = "std")]
use std::io;

/// API building calls to contracts ABI.
#[derive(Clone, Debug, PartialEq)]
pub struct Contract {
	/// Contract constructor.
	pub constructor: Option<Constructor>,
	/// Contract functions.
	pub functions: BTreeMap<String, Vec<Function>>,
	/// Contract events, maps signature to event.
	pub events: BTreeMap<String, Vec<Event>>,
	/// Contract has fallback function.
	pub fallback: bool,
	/// Contract receives.
	pub receive: bool,
}

impl<'a> Deserialize<'a> for Contract {
	fn deserialize<D>(deserializer: D) -> Result<Contract, D::Error>
	where
		D: Deserializer<'a>,
	{
		deserializer.deserialize_any(ContractVisitor)
	}
}

struct ContractVisitor;

impl<'a> Visitor<'a> for ContractVisitor {
	type Value = Contract;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("valid abi spec file")
	}

	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
	where
		A: SeqAccess<'a>,
	{
		let mut result = Contract {
			constructor: None,
			functions: BTreeMap::default(),
			events: BTreeMap::default(),
			fallback: false,
			receive: false,
		};

		while let Some(operation) = seq.next_element()? {
			match operation {
				Operation::Constructor(constructor) => {
					result.constructor = Some(constructor);
				}
				Operation::Function(func) => {
					result.functions.entry(func.name.clone()).or_default().push(func);
				}
				Operation::Event(event) => {
					result.events.entry(event.name.clone()).or_default().push(event);
				}
				Operation::Fallback => {
					result.fallback = true;
				}
				Operation::Receive => {
					result.receive = true;
				}
			}
		}

		Ok(result)
	}
}

impl Contract {
	/// Loads contract from json.
	pub fn from_str(s: &str) -> errors::Result<Self> {
		serde_json::from_str(s).map_err(From::from)
	}

	/// Loads contract from json.
	#[cfg(feature = "std")]
	pub fn load<T: io::Read>(reader: T) -> errors::Result<Self> {
		serde_json::from_reader(reader).map_err(From::from)
	}

	/// Creates constructor call builder.
	pub fn constructor(&self) -> Option<&Constructor> {
		self.constructor.as_ref()
	}

	/// Get the function named `name`, the first if there are overloaded
	/// versions of the same function.
	pub fn function(&self, name: &str) -> errors::Result<&Function> {
		self.functions.get(name).into_iter().flatten().next().ok_or_else(|| Error::InvalidName(name.to_owned()))
	}

	/// Get the contract event named `name`, the first if there are multiple.
	pub fn event(&self, name: &str) -> errors::Result<&Event> {
		self.events.get(name).into_iter().flatten().next().ok_or_else(|| Error::InvalidName(name.to_owned()))
	}

	/// Get all contract events named `name`.
	pub fn events_by_name(&self, name: &str) -> errors::Result<&Vec<Event>> {
		self.events.get(name).ok_or_else(|| Error::InvalidName(name.to_owned()))
	}

	/// Get all functions named `name`.
	pub fn functions_by_name(&self, name: &str) -> errors::Result<&Vec<Function>> {
		self.functions.get(name).ok_or_else(|| Error::InvalidName(name.to_owned()))
	}

	/// Iterate over all functions of the contract in arbitrary order.
	pub fn functions(&self) -> Functions {
		Functions(self.functions.values().flatten())
	}

	/// Iterate over all events of the contract in arbitrary order.
	pub fn events(&self) -> Events {
		Events(self.events.values().flatten())
	}

	/// Returns true if contract has fallback
	pub fn fallback(&self) -> bool {
		self.fallback
	}
}

/// Contract functions iterator.
pub struct Functions<'a>(Flatten<Values<'a, String, Vec<Function>>>);

impl<'a> Iterator for Functions<'a> {
	type Item = &'a Function;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next()
	}
}

/// Contract events iterator.
pub struct Events<'a>(Flatten<Values<'a, String, Vec<Event>>>);

impl<'a> Iterator for Events<'a> {
	type Item = &'a Event;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next()
	}
}
