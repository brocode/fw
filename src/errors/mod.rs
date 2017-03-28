use std::io;
use std::time::SystemTimeError;
use serde_json;
use git2;
use std::ffi::OsString;

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
