USE reminders;

CREATE TABLE reminder_template (
    `id` INT UNSIGNED NOT NULL AUTO_INCREMENT,

    `name` VARCHAR(24) NOT NULL DEFAULT 'Reminder',

    `guild_id` INT UNSIGNED NOT NULL,

    `username` VARCHAR(32) DEFAULT NULL,
    `avatar` VARCHAR(512) DEFAULT NULL,

    `content` VARCHAR(2048) NOT NULL DEFAULT '',
    `tts` BOOL NOT NULL DEFAULT 0,
    `attachment` MEDIUMBLOB,
    `attachment_name` VARCHAR(260),

    `embed_title` VARCHAR(256) NOT NULL DEFAULT '',
    `embed_description` VARCHAR(2048) NOT NULL DEFAULT '',
    `embed_image_url` VARCHAR(512),
    `embed_thumbnail_url` VARCHAR(512),
    `embed_footer` VARCHAR(2048) NOT NULL DEFAULT '',
    `embed_footer_url` VARCHAR(512),
    `embed_author` VARCHAR(256) NOT NULL DEFAULT '',
    `embed_author_url` VARCHAR(512),
    `embed_color` INT UNSIGNED NOT NULL DEFAULT 0x0,
    `embed_fields` JSON,

    PRIMARY KEY (id),

    FOREIGN KEY (`guild_id`) REFERENCES guilds (`id`) ON DELETE CASCADE
);

ALTER TABLE reminders ADD COLUMN embed_fields JSON;
