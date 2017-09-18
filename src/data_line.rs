use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::net::{AddrParseError, IpAddr, Ipv4Addr};
use std::str::FromStr;
use multistr::StringVec;
use multistr::Iter as SVIter;

/// Characters which aren't allowed in URLs.
static INVALID_CHARS: &[char] = &[
    '\0',
    '\u{0009}',
    '\u{000a}',
    '\u{000d}',
    '\u{0020}',
    '#',
    '%',
    '/',
    ':',
    '?',
    '@',
    '[',
    '\\',
    ']',
];

/// Data from a line in `/etc/hosts`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DataLine {
    ip: IpAddr,
    hosts: StringVec,
}
impl DataLine {
    /// Creates a new line from its raw parts.
    pub fn from_raw<'a, I: IntoIterator<Item = &'a str>>(ip: IpAddr, hosts: I) -> DataLine {
        DataLine {
            ip: ip,
            hosts: hosts.into_iter().collect(),
        }
    }

    /// Gets the IP for this line.
    pub fn ip(&self) -> IpAddr {
        self.ip
    }

    /// Iterates over the hosts on this line.
    pub fn hosts(&self) -> Hosts {
        Hosts { inner: Some(self.hosts.iter()) }
    }

    /// Expands this line, iterating over its host/IP pairs.
    pub fn pairs(&self) -> LinePairs {
        LinePairs {
            ip: self.ip,
            hosts: self.hosts.iter(),
        }
    }

    /// Expands this line, iterating over its host/IP pairs. (owned version)
    pub fn into_pairs(self) -> IntoPairs {
        IntoPairs {
            ip: self.ip,
            hosts: self.hosts,
        }
    }
}

/// Minifies a list of data lines.`
pub fn minify_lines(lines: &mut Vec<DataLine>) {
    let mut min = BTreeMap::new();
    for line in lines.drain(..) {
        min.entry(line.ip()).or_insert_with(Vec::new).extend(
            line.hosts().map(ToOwned::to_owned),
        );
    }
    for (ip, mut hosts) in min {
        hosts.sort();
        hosts.dedup();
        lines.push(DataLine::from_raw(ip, hosts.iter().map(|s| &**s)));
    }
}

/// Not actually made public; hack to get `Line::hosts` to work.
pub fn empty_hosts() -> Hosts<'static> {
    Hosts { inner: None }
}
pub fn empty_pairs() -> IntoPairs {
    IntoPairs {
        ip: IpAddr::from([0, 0, 0, 0]),
        hosts: StringVec::new(),
    }
}

/// Iterator over the hosts on a line.
pub struct Hosts<'a> {
    inner: Option<SVIter<'a, str>>,
}
impl<'a> Iterator for Hosts<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        self.inner.as_mut().and_then(|inner| inner.next())
    }
}

/// Iterator over the host/IP pairs on a line.
pub struct LinePairs<'a> {
    ip: IpAddr,
    hosts: SVIter<'a, str>,
}
impl<'a> Iterator for LinePairs<'a> {
    type Item = (&'a str, IpAddr);
    fn next(&mut self) -> Option<(&'a str, IpAddr)> {
        self.hosts.next().map(|h| (h, self.ip))
    }
}

/// Iterator over the host/IP pairs on a line. (owned version)
pub struct IntoPairs {
    ip: IpAddr,
    hosts: StringVec,
}
impl Iterator for IntoPairs {
    type Item = (String, IpAddr);
    fn next(&mut self) -> Option<(String, IpAddr)> {
        self.hosts.pop_off().map(|h| (h, self.ip))
    }
}

/// Error parsing a line in `/etc/hosts`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataParseError {
    /// The line didn't have a space between the host and IP.
    ///
    /// This includes any line that doesn't have an internal space; the host and IP are not actually
    /// checked.
    NoInternalSpace,

    /// The given host was actually an IPv4 address.
    HostWasIp(Ipv4Addr),

    /// The given host had an invalid character.
    BadHost(char, String),

    /// The IP failed to parse.
    BadIp(AddrParseError, String),
}
impl Error for DataParseError {
    fn description(&self) -> &str {
        match *self {
            DataParseError::NoInternalSpace => "line had no space between IP and hosts",
            DataParseError::HostWasIp(_) => "an IP was given where a domain should have been",
            DataParseError::BadHost(_, _) => {
                "a host was invalid because it contains an invalid character"
            }
            DataParseError::BadIp(_, _) => "could not parse IP",
        }
    }
    fn cause(&self) -> Option<&Error> {
        if let DataParseError::BadIp(ref err, _) = *self {
            Some(err)
        } else {
            None
        }
    }
}
impl fmt::Display for DataParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataParseError::NoInternalSpace => write!(f, "line had no space between IP and hosts"),
            DataParseError::HostWasIp(ref ip) => {
                write!(f, "the IP {} was given instead of a domain", ip)
            }
            DataParseError::BadHost(ref ch, ref host) => {
                write!(
                    f,
                    "the host {:?} is invalid because it contains {:?}",
                    host,
                    ch
                )
            }
            DataParseError::BadIp(_, ref ip) => write!(f, "could not parse {:?} as an IP", ip),
        }
    }
}

impl FromStr for DataLine {
    type Err = DataParseError;
    fn from_str(s: &str) -> Result<DataLine, DataParseError> {
        let s = s.trim();
        if let Some(idx) = s.find(char::is_whitespace) {
            let ip = s[..idx].parse().map_err(|err| {
                DataParseError::BadIp(err, s[..idx].to_owned())
            })?;
            let mut hosts = StringVec::new();
            for host in s[idx..].split_whitespace() {
                // https://url.spec.whatwg.org/#host-parsing
                if let Some(idx) = host.find(INVALID_CHARS) {
                    return Err(DataParseError::BadHost(
                        host[idx..].chars().next().unwrap(),
                        host.to_owned(),
                    ));
                } else if let Ok(ipv4) = host.parse::<Ipv4Addr>() {
                    return Err(DataParseError::HostWasIp(ipv4));
                } else {
                    hosts.push(host);
                }
            }
            Ok(DataLine {
                ip: ip,
                hosts: hosts,
            })
        } else {
            Err(DataParseError::NoInternalSpace)
        }
    }
}

impl fmt::Display for DataLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ", self.ip())?;
        for host in self.hosts() {
            write!(f, " {}", host)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use super::*;

    #[test]
    fn only_ip() {
        let line: Result<DataLine, _> = "   ::1   ".parse();
        assert_eq!(line, Err(DataParseError::NoInternalSpace))
    }

    #[test]
    fn wrong_order() {
        let line: Result<DataLine, _> = "localhost ::1".parse();
        if let Err(DataParseError::BadIp(_, ip)) = line {
            assert_eq!(ip, "localhost");
        } else {
            panic!("not a bad IP: {:?}", line);
        }
    }

    #[test]
    fn two_ipv4() {
        let line: Result<DataLine, _> = "127.0.0.1 0.0.0.0".parse();
        if let Err(DataParseError::HostWasIp(ip)) = line {
            assert_eq!(ip, Ipv4Addr::new(0, 0, 0, 0));
        } else {
            panic!("not host-was-IP: {:?}", line);
        }
    }

    #[test]
    fn two_ipv6() {
        let line: Result<DataLine, _> = "::1 localhost ::1".parse();
        if let Err(DataParseError::BadHost(':', host)) = line {
            assert_eq!(host, "::1");
        } else {
            panic!("not a bad host: {:?}", line);
        }
    }

    #[test]
    fn good() {
        let line: DataLine = "::1 localhost localhost.localdomain lh".parse().unwrap();
        assert_eq!(line.ip(), IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));
        let hosts: Vec<&str> = line.hosts().collect();
        assert_eq!(hosts, &["localhost", "localhost.localdomain", "lh"]);
    }

    #[test]
    fn ascii_host() {
        let line: DataLine = "::1 the-quick-brown-fox-jumped-over-the-lazy-dog-0123456789.com"
            .parse()
            .unwrap();
        assert_eq!(line.ip(), IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));
        let hosts: Vec<&str> = line.hosts().collect();
        assert_eq!(
            hosts,
            &[
                "the-quick-brown-fox-jumped-over-the-lazy-dog-0123456789.com",
            ]
        );
    }
}
