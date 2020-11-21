use std::collections::HashMap;

use serde::Deserialize;
use serde_json::from_reader;
use serenity::prelude::TypeMapKey;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Deserialize)]
pub struct LanguageManager {
    languages: HashMap<String, String>,
    strings: HashMap<String, HashMap<String, String>>,
}

impl LanguageManager {
    pub(crate) fn from_compiled<P>(path: P) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let new: Self = from_reader(reader)?;

        Ok(new)
    }

    pub(crate) fn get(&self, language: &str, name: &'static str) -> &str {
        self.strings
            .get(language)
            .map(|sm| sm.get(name))
            .expect(&format!(r#"Language does not exist: "{}""#, language))
            .expect(&format!(r#"String does not exist: "{}""#, name))
    }

    fn all_languages(&self) -> Vec<(&str, &str)> {
        self.languages
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

impl TypeMapKey for LanguageManager {
    type Value = Self;
}
