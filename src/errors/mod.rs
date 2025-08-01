use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum AppError {
	Io(io::Error),
	UserError(String),
	RuntimeError(String),
	BadJson(serde_json::Error),
	InternalError(&'static str),
	GitError(git2::Error),
	Regex(regex::Error),
	TomlSerError(toml::ser::Error),
	TomlDeError(toml::de::Error),
	WalkdirError(walkdir::Error),
	ReqwestError(reqwest::Error),
}

macro_rules! app_error_from {
	($error: ty, $app_error: ident) => {
		impl From<$error> for AppError {
			fn from(err: $error) -> AppError {
				AppError::$app_error(err)
			}
		}
	};
}

impl AppError {
	pub fn require<T>(option: Option<T>, app_error: AppError) -> Result<T, AppError> {
		if let Some(value) = option {
			Result::Ok(value)
		} else {
			Result::Err(app_error)
		}
	}
}

impl fmt::Display for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match *self {
			AppError::Io(ref err) => write!(f, "Io error: {err}"),
			AppError::UserError(ref str) => write!(f, "User error: {str}"),
			AppError::RuntimeError(ref str) => write!(f, "Runtime error: {str}"),
			AppError::BadJson(ref err) => write!(f, "JSON error: {err}"),
			AppError::InternalError(str) => write!(f, "Internal error: {str}"),
			AppError::GitError(ref err) => write!(f, "Git error: {err}"),
			AppError::Regex(ref err) => write!(f, "Regex error: {err}"),
			AppError::TomlSerError(ref err) => write!(f, "toml serialization error: {err}"),
			AppError::TomlDeError(ref err) => write!(f, "toml read error: {err}"),
			AppError::WalkdirError(ref err) => write!(f, "walkdir error: {err}"),
			AppError::ReqwestError(ref err) => write!(f, "reqwest error: {err}"),
		}
	}
}

impl Error for AppError {
	fn cause(&self) -> Option<&dyn Error> {
		match *self {
			AppError::Io(ref err) => Some(err),
			AppError::UserError(_) | AppError::RuntimeError(_) | AppError::InternalError(_) => None,
			AppError::BadJson(ref err) => Some(err),
			AppError::GitError(ref err) => Some(err),
			AppError::Regex(ref err) => Some(err),
			AppError::TomlSerError(ref err) => Some(err),
			AppError::TomlDeError(ref err) => Some(err),
			AppError::WalkdirError(ref err) => Some(err),
			AppError::ReqwestError(ref err) => Some(err),
		}
	}
}

impl From<core::num::ParseIntError> for AppError {
	fn from(err: core::num::ParseIntError) -> AppError {
		AppError::UserError(format!("Type error: {err}"))
	}
}

app_error_from!(git2::Error, GitError);
app_error_from!(io::Error, Io);
app_error_from!(serde_json::Error, BadJson);
app_error_from!(regex::Error, Regex);
app_error_from!(toml::ser::Error, TomlSerError);
app_error_from!(toml::de::Error, TomlDeError);
app_error_from!(walkdir::Error, WalkdirError);
app_error_from!(reqwest::Error, ReqwestError);
