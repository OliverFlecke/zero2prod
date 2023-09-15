-- Add down migration script here
ALTER TABLE subscriptions DROP COLUMN status;
