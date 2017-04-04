use slog::Logger;
use errors::AppError;
use config::Config;
use std::path::Path;
use std::env;
use std::fs;
use std::io;
use std::io::Write;


pub fn projectile(maybe_config: Result<Config, AppError>, logger: &Logger) -> Result<(), AppError> {
  let config: Config = try!(maybe_config);
  let workspace = config.settings.workspace.clone();
  let projects_names: Vec<String> = config
    .projects
    .into_iter()
    .map(|(_, p)| p.name)
    .collect();
  let mut projectile_bookmarks = try!(env::home_dir().ok_or(AppError::UserError("$HOME not set"
                                                                                  .to_owned())));
  projectile_bookmarks.push(".emacs.d");
  projectile_bookmarks.push("projectile-bookmarks.eld");
  let writer = try!(fs::File::create(projectile_bookmarks));
  persist(logger, writer, workspace, projects_names)
}

fn persist<W>(logger: &Logger,
              writer: W,
              workspace: String,
              names: Vec<String>)
              -> Result<(), AppError>
  where W: io::Write
{
  let root: &Path = Path::new(&workspace);
  let paths: Vec<String> = names
    .into_iter()
    .map(|n| root.join(n))
    .flat_map(|path_buf| path_buf.to_str().map(|p| p.to_owned()))
    .collect();
  let mut buffer = io::BufWriter::new(writer);
  try!(buffer.write_all(b"("));
  for path in paths {
    debug!(logger, "Writing projectile entry"; "entry" => path);
    try!(buffer.write_all(format!("\"{}\"", path).as_bytes()));
    try!(buffer.write_all(b" "));
  }
  try!(buffer.write_all(b")"));
  Ok(())
}

#[test]
fn test_persists_projectile_config() {
  use std::io::Cursor;
  use slog_term;
  use slog::{LevelFilter, Level, DrainExt};
  use std::str;
  let mut buffer = Cursor::new(vec![0; 43]);
  let logger = Logger::root(LevelFilter::new(slog_term::StreamerBuilder::new().stdout().build(),
                                             Level::Info)
                                .fuse(),
                            o!());
  let names = vec!["test".to_owned(), "other".to_owned()];

  persist(&logger, &mut buffer, "/home/mriehl".to_owned(), names).unwrap();

  assert_eq!(str::from_utf8(buffer.get_ref()).unwrap(),
             "(\"/home/mriehl/test\" \"/home/mriehl/other\" )");
}
