use error_chain::*;
error_chain! {

  errors {
        UserError(description: String) {
            description(&description)
            display("User error: '{}'", description)
        }
        InternalError(description: String) {
            description(&description)
            display("Internal error: '{}'", description)
        }
        RuntimeError(description: String) {
            description(&description)
            display("Internal error: '{}'", description)
        }
    }
  foreign_links {
    Io(::std::io::Error);
    BadJson(::serde_json::Error);
    Regex(::regex::Error);
    Git(::git2::Error);
    Time(::std::time::SystemTimeError);
    ParseInt(::std::num::ParseIntError);
    GitHub(::github_gql::errors::Error);
  }
}

pub fn fw_require<T>(option: Option<T>, app_error: ErrorKind) -> Result<T> {
  if let Some(value) = option {
    Ok(value)
  } else {
    Err(app_error.into())
  }
}
