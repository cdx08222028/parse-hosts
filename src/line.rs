use std::borrow::Cow;
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;
use super::{DataLine, DataParseError, Hosts};
use super::data_line::empty_hosts;

/// Formatted line in `/etc/hosts`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Line<'a> {
    data: Option<DataLine>,
    comment: Option<Cow<'a, str>>,
}
impl Line<'static> {
    /// Creates an empty line.
    pub fn empty() -> Line<'static> {
        Line {
            data: None,
            comment: None,
        }
    }

    /// Creates a line directly from data
    pub fn from_data(data: DataLine) -> Line<'static> {
        Line {
            data: Some(data),
            comment: None,
        }
    }
}


impl<'a> Line<'a> {
    /// Creates a line from a string.
    pub fn new(line: &str) -> Result<Line, DataParseError> {
        let (comment, stripped) = if let Some(idx) = line.find('#') {
            (Some(Cow::from(line[idx + 1..].trim_left())), &line[..idx])
        } else {
            (None, line)
        };
        let stripped = stripped.trim_right();
        let data = if stripped.is_empty() {
            None
        } else {
            Some(stripped.parse()?)
        };
        Ok(Line {
            data: data,
            comment: comment,
        })
    }

    /// Creates a line directly from a comment.
    pub fn from_comment(comment: &str) -> Line {
        Line {
            data: None,
            comment: Some(comment.into()),
        }
    }

    /// Creates a line from data and a comment.
    pub fn from_raw(data: DataLine, comment: &str) -> Line {
        Line {
            data: Some(data),
            comment: Some(comment.into()),
        }
    }

    /// Gets the IP for this line.
    pub fn ip(&self) -> Option<IpAddr> {
        self.data.as_ref().map(|data| data.ip())
    }

    /// Gets the IP for this line.
    pub fn hosts(&self) -> Hosts {
        if let Some(data) = self.data.as_ref() {
            data.hosts()
        } else {
            empty_hosts()
        }
    }

    /// Gets the data from this line.
    pub fn data(&self) -> Option<&DataLine> {
        self.data.as_ref()
    }

    /// Gets the comment from this line.
    pub fn comment<'b>(&'b self) -> Option<&'b str>
    where
        'a: 'b,
    {
        self.comment.as_ref().map(|s| &**s)
    }

    /// Strips the comment from the line.
    pub fn into_data(self) -> Option<DataLine> {
        self.data
    }

    /// Makes an owned version of the line.
    pub fn into_owned(self) -> Line<'static> {
        Line {
            data: self.data,
            comment: self.comment.map(Cow::into_owned).map(Cow::Owned),
        }
    }
}

impl FromStr for Line<'static> {
    type Err = DataParseError;
    fn from_str(s: &str) -> Result<Line<'static>, DataParseError> {
        Line::new(s).map(Line::into_owned)
    }
}

impl<'a> fmt::Display for Line<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.data(), self.comment()) {
            (Some(data), Some(comment)) => write!(f, "{}  # {}", data, comment),
            (None, Some(comment)) => write!(f, "# {}", comment),
            (Some(data), None) => fmt::Display::fmt(data, f),
            (None, None) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use super::Line;

    #[test]
    fn parse_empty() {
        let empty: Line = "      \t    ".parse().unwrap();
        assert!(empty.comment().is_none());
        assert!(empty.data().is_none());
        assert!(empty.ip().is_none());
        let hosts: Vec<&str> = empty.hosts().collect();
        assert!(hosts.is_empty());
    }

    #[test]
    fn parse_comment() {
        let comment: Line = "   #   \t what? ".parse().unwrap();
        assert_eq!(comment.comment().unwrap(), "what? ");
        assert!(comment.data().is_none());
        assert!(comment.ip().is_none());
        let hosts: Vec<&str> = comment.hosts().collect();
        assert!(hosts.is_empty());
    }

    #[test]
    fn parse_full() {
        let full: Line = "127.0.0.1  \tlocalhost  \t   localhost.localdomain    lh#localhosts"
            .parse()
            .unwrap();
        assert!(full.data().is_some());
        assert_eq!(full.comment().unwrap(), "localhosts");
        assert_eq!(full.ip().unwrap(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let hosts: Vec<&str> = full.hosts().collect();
        assert_eq!(hosts, vec!["localhost", "localhost.localdomain", "lh"]);
    }
}
