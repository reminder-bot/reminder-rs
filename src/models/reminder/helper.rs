use rand::{rngs::OsRng, seq::IteratorRandom};

use crate::consts::CHARACTERS;

pub fn generate_uid() -> String {
    let mut generator: OsRng = Default::default();

    (0..64)
        .map(|_| CHARACTERS.chars().choose(&mut generator).unwrap().to_owned().to_string())
        .collect::<Vec<String>>()
        .join("")
}
