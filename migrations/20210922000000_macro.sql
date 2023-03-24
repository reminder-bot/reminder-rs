CREATE TABLE macro (
    id INT UNSIGNED AUTO_INCREMENT,
    guild_id INT UNSIGNED NOT NULL,

    name VARCHAR(100) NOT NULL,
    description VARCHAR(100),
    commands TEXT NOT NULL,

    FOREIGN KEY (guild_id) REFERENCES guilds(id) ON DELETE CASCADE,
    PRIMARY KEY (id)
);
