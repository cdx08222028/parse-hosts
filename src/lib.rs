#![feature(field_init_shorthand, static_in_const, type_ascription)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(clippy))]

extern crate multistr;

mod data_line;
mod hosts_file;
mod line;
pub use data_line::{DataLine, DataParseError, Hosts};
pub use hosts_file::*;
pub use line::*;
