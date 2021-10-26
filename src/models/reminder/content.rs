pub struct Content {
    pub content: String,
    pub tts: bool,
    pub attachment: Option<Vec<u8>>,
    pub attachment_name: Option<String>,
}

impl Content {
    pub fn new() -> Self {
        Self { content: "".to_string(), tts: false, attachment: None, attachment_name: None }
    }
}
