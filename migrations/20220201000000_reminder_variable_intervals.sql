ALTER TABLE reminders RENAME COLUMN `interval` TO `interval_seconds`;
ALTER TABLE reminders ADD COLUMN `interval_months` INT UNSIGNED DEFAULT NULL;
