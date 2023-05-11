-- Add migration script here
ALTER TABLE reminders ADD COLUMN `thread_id` BIGINT DEFAULT NULL;
