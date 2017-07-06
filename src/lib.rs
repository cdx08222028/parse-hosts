extern crate multistr;

mod data_line;
mod hosts_file;
mod line;
pub use data_line::{DataLine, DataParseError, Hosts, IntoPairs, LinePairs, minify_lines};
pub use hosts_file::*;
pub use line::*;
