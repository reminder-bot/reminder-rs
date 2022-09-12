SET FOREIGN_KEY_CHECKS = 0;

ALTER TABLE channels DROP FOREIGN KEY `channels_ibfk_1`;
ALTER TABLE channels ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE command_aliases DROP FOREIGN KEY `command_aliases_ibfk_1`;
ALTER TABLE command_aliases ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE command_restrictions DROP FOREIGN KEY `command_restrictions_ibfk_1`;
ALTER TABLE command_restrictions ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE events DROP FOREIGN KEY `events_ibfk_1`;
ALTER TABLE events ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE guild_users DROP FOREIGN KEY `guild_users_ibfk_1`;
ALTER TABLE guild_users ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE macro DROP FOREIGN KEY `macro_ibfk_1`;
ALTER TABLE macro ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE roles DROP FOREIGN KEY `roles_ibfk_1`;
ALTER TABLE roles ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE todos DROP FOREIGN KEY `todos_ibfk_2`;
ALTER TABLE todos ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE reminder_template DROP FOREIGN KEY ``
ALTER TABLE roles ADD CONSTRAINT `guild_id_fk`
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE guilds MODIFY `id` BIGINT UNSIGNED NOT NULL;
UPDATE guilds SET `id` = `guild`;
ALTER TABLE guilds DROP COLUMN `guild`;
ALTER TABLE guilds ADD COLUMN `default_channel` BIGINT UNSIGNED;
ALTER TABLE guilds ADD CONSTRAINT `default_channel_fk`
    FOREIGN KEY (`default_channel`)
        REFERENCES channels(`id`)
        ON DELETE SET NULL
        ON UPDATE CASCADE;

ALTER TABLE channels MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE command_aliases MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE command_restrictions MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE events MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE guild_users MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE macro MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE roles MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE todos MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE reminder_template MODIFY `guild_id` BIGINT UNSIGNED;

SET FOREIGN_KEY_CHECKS = 1;
