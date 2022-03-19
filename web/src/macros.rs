macro_rules! check_length {
    ($max:ident, $field:expr) => {
        if $field.len() > $max {
            return json!({ "error": format!("{} exceeded", stringify!($max)) });
        }
    };
    ($max:ident, $field:expr, $($fields:expr),+) => {
        check_length!($max, $field);
        check_length!($max, $($fields),+);
    };
}

macro_rules! check_length_opt {
    ($max:ident, $field:expr) => {
        if let Some(field) = &$field {
            check_length!($max, field);
        }
    };
    ($max:ident, $field:expr, $($fields:expr),+) => {
        check_length_opt!($max, $field);
        check_length_opt!($max, $($fields),+);
    };
}

macro_rules! check_url {
    ($field:expr) => {
        if !($field.starts_with("http://") || $field.starts_with("https://")) {
            return json!({ "error": "URL invalid" });
        }
    };
    ($field:expr, $($fields:expr),+) => {
        check_url!($max, $field);
        check_url!($max, $($fields),+);
    };
}

macro_rules! check_url_opt {
    ($field:expr) => {
        if let Some(field) = &$field {
            check_url!(field);
        }
    };
    ($field:expr, $($fields:expr),+) => {
        check_url_opt!($field);
        check_url_opt!($($fields),+);
    };
}

macro_rules! check_authorization {
    ($cookies:expr, $ctx:expr, $guild:expr) => {
        use serenity::model::id::UserId;

        let user_id = $cookies.get_private("userid").map(|c| c.value().parse::<u64>().ok()).flatten();

        match user_id {
            Some(user_id) => {
                match GuildId($guild).to_guild_cached($ctx) {
                    Some(guild) => {
                        let member = guild.member($ctx, UserId(user_id)).await;

                        match member {
                            Err(_) => {
                                return json!({"error": "User not in guild"})
                            }

                            Ok(_) => {}
                        }
                    }

                    None => {
                        return json!({"error": "Bot not in guild"})
                    }
                }
            }

            None => {
                return json!({"error": "User not authorized"});
            }
        }
    }
}
