use crate::consts::{CHARACTERS, DAY, HOUR, MINUTE};

use num_integer::Integer;

use rand::{rngs::OsRng, seq::IteratorRandom};

pub fn longhand_displacement(seconds: u64) -> String {
    let (days, seconds) = seconds.div_rem(&DAY);
    let (hours, seconds) = seconds.div_rem(&HOUR);
    let (minutes, seconds) = seconds.div_rem(&MINUTE);

    let mut sections = vec![];

    for (var, name) in [days, hours, minutes, seconds]
        .iter()
        .zip(["days", "hours", "minutes", "seconds"].iter())
    {
        if *var > 0 {
            sections.push(format!("{} {}", var, name));
        }
    }

    sections.join(", ")
}

pub fn generate_uid() -> String {
    let mut generator: OsRng = Default::default();

    (0..64)
        .map(|_| {
            CHARACTERS
                .chars()
                .choose(&mut generator)
                .unwrap()
                .to_owned()
                .to_string()
        })
        .collect::<Vec<String>>()
        .join("")
}
