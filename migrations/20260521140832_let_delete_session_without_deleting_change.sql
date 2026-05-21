-- Add migration script here
ALTER TABLE changes
DROP CONSTRAINT changes_session_id_fkey;

ALTER TABLE changes
ADD CONSTRAINT changes_session_id_fkey
FOREIGN KEY (session_id)
REFERENCES sessions(id)
ON DELETE SET NULL;