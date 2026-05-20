-- Add migration script here
ALTER TABLE changes
    ADD COLUMN prev_classroom TEXT,
    ADD COLUMN new_classroom  TEXT;