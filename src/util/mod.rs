use ansi_term::Colour;

use rand::seq::SliceRandom;

use std::borrow::ToOwned;

use slog::debug;
use slog::Logger;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::{Format, Severity};
use sloggers::Build;

pub static COLOURS: [Colour; 14] = [
  Colour::Green,
  Colour::Cyan,
  Colour::Blue,
  Colour::Yellow,
  Colour::RGB(255, 165, 0),
  Colour::RGB(255, 99, 71),
  Colour::RGB(0, 153, 255),
  Colour::RGB(102, 0, 102),
  Colour::RGB(102, 0, 0),
  Colour::RGB(153, 102, 51),
  Colour::RGB(102, 153, 0),
  Colour::RGB(0, 0, 102),
  Colour::RGB(255, 153, 255),
  Colour::Purple,
];

pub fn random_colour() -> Colour {
  let mut rng = rand::thread_rng();
  COLOURS.choose(&mut rng).map(ToOwned::to_owned).unwrap_or(Colour::Black)
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

  debug!(logger, "Logger ready" ; "level" => format!("{:?}", log_level));
  logger
}
