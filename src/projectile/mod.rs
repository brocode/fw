use config;
use config::Config;
use errors::AppError;
use slog::Logger;
use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;


pub fn projectile(maybe_config: Result<Config, AppError>, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  let workspace = config.settings.workspace.clone();
  let projects_paths: Vec<PathBuf> = config.projects
                                           .into_iter()
                                           .map(|(_, p)| config::actual_path_to_project(&workspace, &p))
                                           .collect();
  let mut projectile_bookmarks = env::home_dir()
    .ok_or_else(|| AppError::UserError("$HOME not set".to_owned()))?;
  projectile_bookmarks.push(".emacs.d");
  projectile_bookmarks.push("projectile-bookmarks.eld");
  let writer = fs::File::create(projectile_bookmarks)?;
  persist(logger, writer, projects_paths)
}

fn persist<W>(logger: &Logger, writer: W, paths: Vec<PathBuf>) -> Result<(), AppError>
  where W: io::Write
{
  let paths: Vec<String> = paths.into_iter()
                                .flat_map(|path_buf| path_buf.to_str().map(|p| p.to_owned()))
                                .collect();
  let mut buffer = io::BufWriter::new(writer);
  buffer.write_all(b"(")?;
  for path in paths {
    debug!(logger, "Writing projectile entry"; "entry" => path);
    buffer.write_all(format!("\"{}\"", path).as_bytes())?;
    buffer.write_all(b" ")?;
  }
  buffer.write_all(b")")?;
  Ok(())
}

#[test]
fn test_persists_projectile_config() {
  use std::io::Cursor;
  use slog_term;
  use slog::{DrainExt, Level, LevelFilter};
  use std::str;
  let mut buffer = Cursor::new(vec![0; 61]);
  let logger = Logger::root(LevelFilter::new(slog_term::StreamerBuilder::new().stdout().build(),
                                             Level::Info)
                              .fuse(),
                            o!());
  let paths = vec![PathBuf::from("/home/mriehl/test"),
                   PathBuf::from("/home/mriehl/go/src/github.com/test2")];

  persist(&logger, &mut buffer, paths).unwrap();

  assert_eq!(str::from_utf8(buffer.get_ref()).unwrap(),
             "(\"/home/mriehl/test\" \"/home/mriehl/go/src/github.com/test2\" )");
}
