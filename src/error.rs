use std::io;
use std::error;
use std::fmt;
use std::path::PathBuf;
use std::result;

use regex;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Regex(regex::Error),
    Io(io::Error),
    OpenFile {
        path: PathBuf,
        err: io::Error,
    },
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Regex(ref err) => err.description(),
            Error::Io(ref err) => err.description(),
            Error::OpenFile { ref err, .. } => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        Some(match *self {
            Error::Regex(ref err) => err,
            Error::Io(ref err) => err,
            Error::OpenFile { ref err, .. } => err,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Regex(ref err) => err.fmt(f),
            Error::Io(ref err) => err.fmt(f),
            Error::OpenFile { ref path, ref err } => {
                write!(f, "{}: {}", err, path.display())
            }
        }
    }
}

impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Error {
        Error::Regex(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
