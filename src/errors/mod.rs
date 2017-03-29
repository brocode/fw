use std::io;
use std::time::SystemTimeError;
use serde_json;
use git2;
use std::ffi::OsString;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
  IO(io::Error),
  UserError(String),
  BadJson(serde_json::Error),
  InternalError(&'static str),
  ClockError(SystemTimeError),
  GitError(git2::Error),
  Utf8Error(OsString),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppError::IO(ref err) => write!(f, "IO error: {}", err),
            AppError::UserError(ref str) => write!(f, "User error: {}", str),
            AppError::BadJson(ref err) => write!(f, "JSON error: {}", err),
            AppError::InternalError(ref str) => write!(f, "Internal error: {}", str),
            AppError::ClockError(ref err) => write!(f, "System clock error: {}", err),
            AppError::GitError(ref err) => write!(f, "Git error: {}", err),
            AppError::Utf8Error(ref err) => write!(f, "UTF8 conversion error: {:?}", err),
        }
    }
}

impl error::Error for AppError {
    fn description(&self) -> &str {
        match *self {
            AppError::IO(ref err) => err.description(),
            AppError::UserError(ref str) => str.as_ref(),
            AppError::BadJson(ref err) => err.description(),
            AppError::InternalError(ref str) => str.as_ref(),
            AppError::ClockError(ref err) => err.description(),
            AppError::GitError(ref err) => err.description(),
            AppError::Utf8Error(ref os_str) => "invalid utf8",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            AppError::IO(ref err) => Some(err),
            AppError::UserError(_) => None,
            AppError::BadJson(ref err) => Some(err),
            AppError::InternalError(_) => None,
            AppError::ClockError(ref err) => Some(err),
            AppError::GitError(ref err) => Some(err),
            AppError::Utf8Error(_) => None,
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
