//! # lorri
//! lorri is a wrapper over Nix to abstract project-specific build
//! configuration and patterns in to a declarative configuration.

#![warn(missing_docs)]

#[macro_use]
extern crate structopt;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate regex;
#[macro_use]
extern crate lazy_static;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate directories;
extern crate futures;
extern crate notify;
extern crate tempfile;
extern crate vec1;

extern crate proptest;

pub mod bash;
pub mod build;
pub mod build_loop;
pub mod builder;
pub mod changelog;
pub mod cli;
pub mod communicate;
pub mod locate_file;
pub mod logging;
pub mod mpsc;
pub mod nix;
pub mod ops;
pub mod pathreduction;
pub mod project;
pub mod roots;
pub mod watch;

// OUT_DIR and build_rev.rs are generated by cargo, see ../build.rs
include!(concat!(env!("OUT_DIR"), "/build_rev.rs"));
