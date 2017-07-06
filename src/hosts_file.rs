use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::net::IpAddr;
use super::{DataLine, DataParseError, Line, IntoPairs};
use super::data_line::empty_pairs;

/// Shorthand for `HostsFile<BufReader<R>>`.
pub type BufHostsFile<R: Read> = HostsFile<BufReader<R>>;

/// Shorthand for `HostsFile<BufReader<File>>`.
pub type ActualHostsFile = HostsFile<BufReader<File>>;

/// Representation of `/etc/hosts`.
pub struct HostsFile<R: BufRead> {
    inner: R,
}

impl HostsFile<BufReader<File>> {
    /// Loads the data from `/etc/hosts`.
    pub fn load() -> io::Result<HostsFile<BufReader<File>>> {
        Ok(HostsFile { inner: BufReader::new(File::open("/etc/hosts")?) })
    }
}
impl<R: Read> HostsFile<BufReader<R>> {
    /// Loads the data from `/etc/hosts` from a generic reader wrapped in a `BufReader`.
    pub fn read_buffered(reader: R) -> HostsFile<BufReader<R>> {
        HostsFile { inner: BufReader::new(reader) }
    }
}
impl<R: BufRead> HostsFile<R> {
    /// Loads the data from `/etc/hosts` from a generic reader.
    pub fn read(reader: R) -> HostsFile<R> {
        HostsFile { inner: reader }
    }

    /// Iterates over all lines in the file.
    pub fn lines(self) -> Lines<R> {
        Lines { inner: self.inner.lines() }
    }

    /// Iterates over the lines in the file with data.
    pub fn data_lines(self) -> DataLines<R> {
        DataLines { inner: self.inner.lines() }
    }

    /// Iterates over the IP/host pairs in the file.
    pub fn pairs(self) -> Pairs<R> {
        Pairs { inner: self.data_lines(), pairs: empty_pairs() }
    }
}

/// Error found when reading a line in `/etc/hosts`.
#[derive(Debug)]
pub enum LineReadError {
    /// The line failed to read.
    Read(io::Error),

    /// The line failed to parse.
    Parse(DataParseError),
}
impl From<io::Error> for LineReadError {
    fn from(err: io::Error) -> LineReadError {
        LineReadError::Read(err)
    }
}
impl From<DataParseError> for LineReadError {
    fn from(err: DataParseError) -> LineReadError {
        LineReadError::Parse(err)
    }
}
impl Error for LineReadError {
    fn description(&self) -> &str {
        match *self {
            LineReadError::Read(ref err) => err.description(),
            LineReadError::Parse(ref err) => err.description(),
        }
    }
    fn cause(&self) -> Option<&Error> {
        Some(match *self {
            LineReadError::Read(ref err) => err,
            LineReadError::Parse(ref err) => err,
        })
    }
}
impl fmt::Display for LineReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LineReadError::Read(ref err) => fmt::Display::fmt(err, f),
            LineReadError::Parse(ref err) => fmt::Display::fmt(err, f),
        }
    }
}

/// Iterator over the lines in `/etc/hosts`.
pub struct Lines<R: BufRead> {
    inner: io::Lines<R>,
}
impl<R: BufRead> Iterator for Lines<R> {
    type Item = Result<Line<'static>, LineReadError>;
    fn next(&mut self) -> Option<Result<Line<'static>, LineReadError>> {
        self.inner.next().map(|line| match line {
            Err(err) => Err(err.into()),
            Ok(line) => line.parse().map_err(Into::into).map(Line::into_owned),
        })
    }
}

/// Iterator over the lines in `/etc/hosts` with data.
pub struct DataLines<R: BufRead> {
    inner: io::Lines<R>,
}
impl<R: BufRead> Iterator for DataLines<R> {
    type Item = Result<DataLine, LineReadError>;
    fn next(&mut self) -> Option<Result<DataLine, LineReadError>> {
        for line in self.inner.by_ref() {
            match line {
                Err(err) => return Some(Err(err.into())),
                Ok(line) => {
                    match line.parse().map(Line::into_data) {
                        Ok(None) => (),
                        Ok(Some(data)) => return Some(Ok(data)),
                        Err(err) => return Some(Err(err.into())),
                    }
                }
            }
        }
        None
    }
}

/// Iterator over the host/IP pairs in `/etc/hosts`.
pub struct Pairs<R: BufRead> {
    inner: DataLines<R>,
    pairs: IntoPairs,
}
impl<R: BufRead> Iterator for Pairs<R> {
    type Item = Result<(String, IpAddr), LineReadError>;
    fn next(&mut self) -> Option<Result<(String, IpAddr), LineReadError>> {
        loop {
            if let Some(next) = self.pairs.next() {
                return Some(Ok(next));
            }
            match self.inner.next() {
                Some(Ok(line)) => {
                    self.pairs = line.into_pairs();
                },
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fmt::Write;
    use super::*;

    static PRETTY: &str = "\
# basic ones
127.0.0.1  localhost localhost.localdomain
0.0.0.0  allzeros  # nonstandard

# others
8.8.8.8  gdns  # this is the more common one
8.8.4.4  gdns2  # this is the less common one

# comment by itself
";

    static PLAIN: &str = "\
127.0.0.1  localhost localhost.localdomain
0.0.0.0  allzeros
8.8.8.8  gdns
8.8.4.4  gdns2
";

    #[test]
    fn lines() {
        let mut rewritten = String::new();
        for line in HostsFile::read_buffered(PRETTY.as_bytes()).lines() {
            let line = line.unwrap();
            writeln!(rewritten, "{}", line).unwrap();
        }
        assert_eq!(rewritten, PRETTY);
    }

    #[test]
    fn data_lines() {
        let mut rewritten = String::new();
        for line in HostsFile::read_buffered(PRETTY.as_bytes()).data_lines() {
            let line = line.unwrap();
            writeln!(rewritten, "{}", line).unwrap();
        }
        assert_eq!(rewritten, PLAIN);
    }

    #[test]
    fn pairs() {
        let mut map = HashMap::new();
        map.extend(HostsFile::read_buffered(PRETTY.as_bytes()).pairs().map(Result::unwrap));
        assert_eq!(*map.get("localhost").unwrap(), "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(*map.get("localhost.localdomain").unwrap(), "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(*map.get("allzeros").unwrap(), "0.0.0.0".parse::<IpAddr>().unwrap());
        assert_eq!(*map.get("gdns").unwrap(), "8.8.8.8".parse::<IpAddr>().unwrap());
        assert_eq!(*map.get("gdns2").unwrap(), "8.8.4.4".parse::<IpAddr>().unwrap());
    }
}
