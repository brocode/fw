use core;
use git2;
use serde_json;
use std::error;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::time::SystemTimeError;

#[derive(Debug)]
pub enum AppError {
  IO(io::Error),
  UserError(String),
  BadJson(serde_json::Error),
  InternalError(&'static str),
  ClockError(SystemTimeError),
  GitError(git2::Error),
  Utf8Error(OsString),
  Utf8ConversionError(core::str::Utf8Error),
}

impl fmt::Display for AppError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
    AppError::IO(ref err) => write!(f, "IO error: {}", err),
    AppError::UserError(ref str) => write!(f, "User error: {}", str),
    AppError::BadJson(ref err) => write!(f, "JSON error: {}", err),
    AppError::InternalError(str) => write!(f, "Internal error: {}", str),
    AppError::ClockError(ref err) => write!(f, "System clock error: {}", err),
    AppError::GitError(ref err) => write!(f, "Git error: {}", err),
    AppError::Utf8Error(ref err) => write!(f, "UTF8 conversion error: {:?}", err),
    AppError::Utf8ConversionError(ref err) => write!(f, "UTF8 conversion error: {:?}", err),
    }
  }
}

impl error::Error for AppError {
  fn description(&self) -> &str {
    match *self {
    AppError::IO(ref err) => err.description(),
    AppError::UserError(ref str) => str.as_ref(),
    AppError::BadJson(ref err) => err.description(),
    AppError::InternalError(str) => str.as_ref(),
    AppError::ClockError(ref err) => err.description(),
    AppError::GitError(ref err) => err.description(),
    AppError::Utf8Error(_) => "invalid utf8",
    AppError::Utf8ConversionError(ref err) => err.description(),
    }
  }

  fn cause(&self) -> Option<&error::Error> {
    match *self {
    AppError::IO(ref err) => Some(err),
    AppError::UserError(_) |
    AppError::InternalError(_) |
    AppError::Utf8Error(_) => None,
    AppError::BadJson(ref err) => Some(err),
    AppError::ClockError(ref err) => Some(err),
    AppError::GitError(ref err) => Some(err),
    AppError::Utf8ConversionError(ref err) => Some(err),
    }
  }
}


impl From<git2::Error> for AppError {
  fn from(err: git2::Error) -> AppError {
    AppError::GitError(err)
  }
}

impl From<io::Error> for AppError {
  fn from(err: io::Error) -> AppError {
    AppError::IO(err)
  }
}

impl From<serde_json::Error> for AppError {
  fn from(err: serde_json::Error) -> AppError {
    AppError::BadJson(err)
  }
}

impl From<core::str::Utf8Error> for AppError {
  fn from(err: core::str::Utf8Error) -> AppError {
    AppError::Utf8ConversionError(err)
  }
}
