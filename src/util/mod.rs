use yansi::Color;

use rand::seq::IndexedRandom;

use std::borrow::ToOwned;

pub static COLOURS: [Color; 12] = [
	Color::Green,
	Color::Cyan,
	Color::Blue,
	Color::Yellow,
	Color::Red,
	Color::Rgb(255, 165, 0),
	Color::Rgb(255, 99, 71),
	Color::Rgb(0, 153, 255),
	Color::Rgb(153, 102, 51),
	Color::Rgb(102, 153, 0),
	Color::Rgb(255, 153, 255),
	Color::Magenta,
];

pub fn random_color() -> Color {
	let mut rng = rand::rng();
	COLOURS.choose(&mut rng).map(ToOwned::to_owned).unwrap_or(Color::Black)
}
