USE reminders;

ALTER TABLE reminders.reminders ADD COLUMN `interval_days` INT UNSIGNED DEFAULT NULL;
