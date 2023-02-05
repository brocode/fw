use yansi::Color;

use rand::seq::SliceRandom;

use std::borrow::ToOwned;

use slog::debug;
use slog::Logger;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::{Format, Severity};
use sloggers::Build;

pub static COLOURS: [Color; 14] = [
  Color::Green,
  Color::Cyan,
  Color::Blue,
  Color::Yellow,
  Color::RGB(255, 165, 0),
  Color::RGB(255, 99, 71),
  Color::RGB(0, 153, 255),
  Color::RGB(102, 0, 102),
  Color::RGB(102, 0, 0),
  Color::RGB(153, 102, 51),
  Color::RGB(102, 153, 0),
  Color::RGB(0, 0, 102),
  Color::RGB(255, 153, 255),
  Color::Magenta,
];

pub fn random_color() -> Color {
  let mut rng = rand::thread_rng();
  COLOURS.choose(&mut rng).map(ToOwned::to_owned).unwrap_or(Color::Black)
}

pub fn logger_from_verbosity(verbosity: u64, quiet: bool) -> Logger {
  let log_level = match verbosity {
    _ if quiet => Severity::Error,
    0 => Severity::Warning,
    1 => Severity::Info,
    2 => Severity::Debug,
    3 => Severity::Trace,
    _ => Severity::Trace,
  };

  let mut logger_builder = TerminalLoggerBuilder::new();
  logger_builder.level(log_level);
  logger_builder.destination(Destination::Stderr);
  logger_builder.format(Format::Full);

  let logger = logger_builder.build().unwrap();

  debug!(logger, "Logger ready" ; "level" => format!("{log_level:?}"));
  logger
}
