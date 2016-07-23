//! A simple Error enum to unify regex and std::io errors. Nothing special here.

use regex;
use std::io;
use std::error;
use std::fmt;
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    RegexError(regex::Error),
    IOError(io::Error),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.cause().unwrap().description()
    }

    fn cause(&self) -> Option<&error::Error> {
        Some(match *self {
            Error::RegexError(ref err) => err,
            Error::IOError(ref err) => err,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        use std::fmt::Display;

        Display::fmt(self.cause().unwrap(), f)
    }
}

impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Error {
        Error::RegexError(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IOError(err)
    }
}
