-- Add down migration script here
ALTER TABLE subscriptions ALTER COLUMN status DROP NOT NULL;
