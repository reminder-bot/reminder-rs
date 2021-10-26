CREATE TABLE macro (
    id INT UNSIGNED AUTO_INCREMENT,
    guild_id BIGINT UNSIGNED NOT NULL,

    name VARCHAR(100) NOT NULL,
    description VARCHAR(100),
    commands TEXT NOT NULL,

    FOREIGN KEY (guild_id) REFERENCES guilds(guild) ON DELETE CASCADE,
    PRIMARY KEY (id)
);

DROP TABLE IF EXISTS events;

CREATE TABLE reminders.todos_new (
    id INT UNSIGNED AUTO_INCREMENT UNIQUE NOT NULL,
    user_id BIGINT UNSIGNED,
    guild_id BIGINT UNSIGNED,
    channel_id BIGINT UNSIGNED,
    value VARCHAR(2000) NOT NULL,

    PRIMARY KEY (id),
    INDEX (user_id),
    INDEX (guild_id),
    INDEX (channel_id)
);

INSERT INTO reminders.todos_new (user_id, guild_id, channel_id, value)
    SELECT users.user, guilds.guild, channels.channel, todos.value
    FROM todos
        INNER JOIN users ON users.id = todos.user_id
        INNER JOIN guilds ON guilds.id = todos.guild_id
        INNER JOIN channels ON channels.id = todos.channel_id;
