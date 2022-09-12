SET foreign_key_checks = 0;

START TRANSACTION;

-- drop existing constraints
ALTER TABLE channels DROP FOREIGN KEY `channels_ibfk_1`;
ALTER TABLE command_aliases DROP FOREIGN KEY `command_aliases_ibfk_1`;
ALTER TABLE events DROP FOREIGN KEY `events_ibfk_1`;
ALTER TABLE guild_users DROP FOREIGN KEY `guild_users_ibfk_1`;
ALTER TABLE macro DROP FOREIGN KEY `macro_ibfk_1`;
ALTER TABLE roles DROP FOREIGN KEY `roles_ibfk_1`;
ALTER TABLE todos DROP FOREIGN KEY `todos_ibfk_2`;
ALTER TABLE reminder_template DROP FOREIGN KEY `reminder_template_ibfk_1`;

-- update foreign key types
ALTER TABLE channels MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE command_aliases MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE events MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE guild_users MODIFY `guild` BIGINT UNSIGNED;
ALTER TABLE macro MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE roles MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE todos MODIFY `guild_id` BIGINT UNSIGNED;
ALTER TABLE reminder_template MODIFY `guild_id` BIGINT UNSIGNED;

-- update foreign key values
UPDATE channels SET `guild_id` = (SELECT `guild` FROM guilds WHERE guilds.`id` = `guild_id`);
UPDATE command_aliases SET `guild_id` = (SELECT `guild` FROM guilds WHERE guilds.`id` = `guild_id`);
UPDATE events SET `guild_id` = (SELECT `guild` FROM guilds WHERE guilds.`id` = `guild_id`);
UPDATE guild_users SET `guild` = (SELECT `guild` FROM guilds WHERE guilds.`id` = guild_users.`guild`);
UPDATE macro SET `guild_id` = (SELECT `guild` FROM guilds WHERE guilds.`id` = `guild_id`);
UPDATE roles SET `guild_id` = (SELECT `guild` FROM guilds WHERE guilds.`id` = `guild_id`);
UPDATE todos SET `guild_id` = (SELECT `guild` FROM guilds WHERE guilds.`id` = `guild_id`);
UPDATE reminder_template SET `guild_id` = (SELECT `guild` FROM guilds WHERE guilds.`id` = `guild_id`);

-- update guilds table
ALTER TABLE guilds MODIFY `id` BIGINT UNSIGNED NOT NULL;
UPDATE guilds SET `id` = `guild`;
ALTER TABLE guilds DROP COLUMN `guild`;
ALTER TABLE guilds ADD COLUMN `default_channel` BIGINT UNSIGNED;
ALTER TABLE guilds ADD CONSTRAINT `default_channel_fk`
    FOREIGN KEY (`default_channel`)
        REFERENCES channels(`channel`)
        ON DELETE SET NULL
        ON UPDATE CASCADE;

-- re-add constraints
ALTER TABLE channels ADD CONSTRAINT
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE command_aliases ADD CONSTRAINT
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE events ADD CONSTRAINT
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE guild_users ADD CONSTRAINT
    FOREIGN KEY (`guild`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE macro ADD CONSTRAINT
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE roles ADD CONSTRAINT
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;

ALTER TABLE todos ADD CONSTRAINT
    FOREIGN KEY (`guild_id`)
        REFERENCES guilds(`id`)
        ON DELETE CASCADE
        ON UPDATE CASCADE;


COMMIT;

SET foreign_key_checks = 1;
