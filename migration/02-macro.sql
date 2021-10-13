CREATE TABLE macro (
    id INT UNSIGNED AUTO_INCREMENT,
    guild_id BIGINT UNSIGNED NOT NULL,

    name VARCHAR(100) NOT NULL,
    description VARCHAR(100),
    commands TEXT NOT NULL,

    FOREIGN KEY (guild_id) REFERENCES guilds(guild) ON DELETE CASCADE,
    PRIMARY KEY (id)
);
