use serde::Deserialize;
use serde_json::from_str;
use serenity::prelude::TypeMapKey;

use std::{collections::HashMap, error::Error, sync::Arc};

use crate::consts::LOCAL_LANGUAGE;

#[derive(Deserialize)]
pub struct LanguageManager {
    languages: HashMap<String, String>,
    strings: HashMap<String, HashMap<String, String>>,
}

impl LanguageManager {
    pub fn from_compiled(content: &'static str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let new: Self = from_str(content)?;

        Ok(new)
    }

    pub fn get(&self, language: &str, name: &str) -> &str {
        self.strings
            .get(language)
            .map(|sm| sm.get(name))
            .unwrap_or_else(|| panic!(r#"Language does not exist: "{}""#, language))
            .unwrap_or_else(|| {
                self.strings
                    .get(&*LOCAL_LANGUAGE)
                    .map(|sm| {
                        sm.get(name)
                            .unwrap_or_else(|| panic!(r#"String does not exist: "{}""#, name))
                    })
                    .expect("LOCAL_LANGUAGE is not available")
            })
    }

    pub fn get_language(&self, language: &str) -> Option<&str> {
        let language_normal = language.to_lowercase();

        self.languages
            .iter()
            .filter(|(k, v)| {
                k.to_lowercase() == language_normal || v.to_lowercase() == language_normal
            })
            .map(|(k, _)| k.as_str())
            .next()
    }

    pub fn get_language_by_flag(&self, flag: &str) -> Option<&str> {
        self.languages
            .iter()
            .filter(|(k, _)| self.get(k, "flag") == flag)
            .map(|(k, _)| k.as_str())
            .next()
    }

    pub fn all_languages(&self) -> impl Iterator<Item = (&str, &str)> {
        self.languages.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl TypeMapKey for LanguageManager {
    type Value = Arc<Self>;
}
