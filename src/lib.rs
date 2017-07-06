#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(clippy))]

extern crate multistr;

mod data_line;
mod hosts_file;
mod line;
pub use data_line::{DataLine, DataParseError, Hosts, minify_lines};
pub use hosts_file::*;
pub use line::*;
