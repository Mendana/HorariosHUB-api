-- Al eliminar un usuario, elimina en cascada sus datos asociados
-- en vez de bloquear el DELETE con una violación de FK.

ALTER TABLE schedule
DROP CONSTRAINT schedule_user_id_fkey;

ALTER TABLE schedule
ADD CONSTRAINT schedule_user_id_fkey
FOREIGN KEY (user_id)
REFERENCES users(id)
ON DELETE CASCADE;

ALTER TABLE changes
DROP CONSTRAINT changes_proposed_by_fkey;

ALTER TABLE changes
ADD CONSTRAINT changes_proposed_by_fkey
FOREIGN KEY (proposed_by)
REFERENCES users(id)
ON DELETE CASCADE;

ALTER TABLE changes_history
DROP CONSTRAINT changes_history_proposed_by_fkey;

ALTER TABLE changes_history
ADD CONSTRAINT changes_history_proposed_by_fkey
FOREIGN KEY (proposed_by)
REFERENCES users(id)
ON DELETE CASCADE;

-- sessions.created_by usa SET NULL en vez de CASCADE para no perder
-- sesiones manuales al borrar al profesor que las creó. Como el CHECK
-- chk_manual_has_creator exige que las sesiones manuales tengan autor,
-- guardamos también el email en el momento de la creación como snapshot
-- permanente, para poder identificar al autor incluso tras borrar su cuenta.
ALTER TABLE sessions
ADD COLUMN created_by_email TEXT;

UPDATE sessions
SET created_by_email = u.email
FROM users u
WHERE sessions.created_by = u.id;

ALTER TABLE sessions
DROP CONSTRAINT chk_manual_has_creator;

ALTER TABLE sessions
ADD CONSTRAINT chk_manual_has_creator
CHECK (source != 'manual' OR created_by_email IS NOT NULL);

ALTER TABLE sessions
DROP CONSTRAINT sessions_created_by_fkey;

ALTER TABLE sessions
ADD CONSTRAINT sessions_created_by_fkey
FOREIGN KEY (created_by)
REFERENCES users(id)
ON DELETE SET NULL;
