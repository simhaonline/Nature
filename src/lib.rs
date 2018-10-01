#![feature(plugin, proc_macro_gen)]
#![plugin(rocket_codegen)]
#![feature(range_contains)]
#![feature(trace_macros)]
#![feature(try_from)] // vec to array
#![feature(box_patterns)]

extern crate chrono;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate lru_time_cache;
extern crate nature_common;
extern crate nature_db;
extern crate rocket;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

pub mod system;
pub mod rpc;
pub mod flow;
pub mod support;
