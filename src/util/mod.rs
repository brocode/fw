use yansi::Color;

use rand::seq::SliceRandom;

use std::borrow::ToOwned;

pub static COLOURS: [Color; 12] = [
    Color::Green,
    Color::Cyan,
    Color::Blue,
    Color::Yellow,
    Color::Red,
    Color::RGB(255, 165, 0),
    Color::RGB(255, 99, 71),
    Color::RGB(0, 153, 255),
    Color::RGB(153, 102, 51),
    Color::RGB(102, 153, 0),
    Color::RGB(255, 153, 255),
    Color::Magenta,
];

pub fn random_color() -> Color {
    let mut rng = rand::thread_rng();
    COLOURS.choose(&mut rng).map(ToOwned::to_owned).unwrap_or(Color::Black)
}
