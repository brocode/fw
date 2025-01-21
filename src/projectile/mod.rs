use crate::config::Config;
use crate::errors::AppError;
use regex::Regex;
use std::borrow::ToOwned;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn projectile(maybe_config: Result<Config, AppError>) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	let projects_paths: Vec<PathBuf> = config.projects.values().map(|p| config.actual_path_to_project(p)).collect();
	let home_dir: PathBuf = dirs::home_dir().ok_or_else(|| AppError::UserError("$HOME not set".to_owned()))?;
	let mut projectile_bookmarks: PathBuf = home_dir.clone();
	projectile_bookmarks.push(".emacs.d");
	projectile_bookmarks.push("projectile-bookmarks.eld");
	let writer = fs::File::create(projectile_bookmarks)?;
	persist(&home_dir, writer, projects_paths)
}

fn persist<W>(home_dir: &Path, writer: W, paths: Vec<PathBuf>) -> Result<(), AppError>
where
	W: io::Write,
{
	let paths: Vec<String> = paths.into_iter().flat_map(|path_buf| path_buf.to_str().map(ToOwned::to_owned)).collect();
	let mut buffer = io::BufWriter::new(writer);
	buffer.write_all(b"(")?;
	for path in paths {
		let path = replace_path_with_tilde(&path, home_dir.to_path_buf()).unwrap_or(path);
		buffer.write_all(format!("\"{}/\"", path).as_bytes())?;
		buffer.write_all(b" ")?;
	}
	buffer.write_all(b")")?;
	Ok(())
}

fn replace_path_with_tilde(path: &str, path_to_replace: PathBuf) -> Result<String, AppError> {
	let replace_string = path_to_replace.into_os_string().into_string().expect("path should be a valid string");
	let mut pattern: String = "^".to_string();
	pattern.push_str(&replace_string);
	let regex = Regex::new(&pattern)?;
	Ok(regex.replace_all(path, "~").into_owned())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	#[test]
	fn test_persists_projectile_config() {
		use std::io::Cursor;
		use std::str;
		let mut buffer = Cursor::new(vec![0; 61]);
		let paths = vec![PathBuf::from("/home/mriehl/test"), PathBuf::from("/home/mriehl/go/src/github.com/test2")];

		let home_dir = Path::new("/home/blubb").to_path_buf();
		persist(&home_dir, &mut buffer, paths).unwrap();

		assert_eq!(
			str::from_utf8(buffer.get_ref()).unwrap(),
			"(\"/home/mriehl/test/\" \"/home/mriehl/go/src/github.com/test2/\" )"
		);
	}

	#[test]
	fn test_replace_path_with_tilde() {
		let home_dir = Path::new("/home/blubb").to_path_buf();

		let replaced_string = replace_path_with_tilde("/home/blubb/moep/home/blubb/test.txt", home_dir).expect("should succeed");
		assert_eq!(replaced_string, "~/moep/home/blubb/test.txt".to_string());
	}
}
