USE reminders;

ALTER TABLE reminders.reminders RENAME COLUMN `interval` TO `interval_seconds`;
ALTER TABLE reminders.reminders ADD COLUMN `interval_months` INT UNSIGNED DEFAULT NULL;
