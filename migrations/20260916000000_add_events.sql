-- subject_groups.id se añadió en 20260524143856 sin restricción de unicidad:
-- cualquier FK futura contra esa columna (como event_groups más abajo, si en el
-- futuro se quisiera referenciar por id en vez de por (subject, grp)) fallaría
-- o correría el riesgo de colisiones no detectadas. Lo corregimos aquí.
ALTER TABLE subject_groups
  ADD CONSTRAINT subject_groups_id_key UNIQUE (id);

CREATE TYPE recurrence_interval AS ENUM ('daily', 'weekly', 'biweekly', 'monthly');

-- Un evento pertenece siempre a una única asignatura (`subject`). Si no tiene
-- filas en `event_groups` aplica a TODOS los grupos de esa asignatura; si tiene
-- una o más filas, aplica solo a esos grupos concretos.
CREATE TABLE events (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title                 TEXT NOT NULL,
    description           TEXT,
    subject               TEXT NOT NULL,
    starts_at             TIMESTAMPTZ NOT NULL,
    duration_min          INTEGER NOT NULL CHECK (duration_min > 0),
    classroom             TEXT,
    created_by            UUID REFERENCES users(id) ON DELETE SET NULL,
    created_by_email      TEXT NOT NULL,
    recurrence_interval   recurrence_interval,
    recurrence_end_date   DATE,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        (recurrence_interval IS NULL AND recurrence_end_date IS NULL)
        OR (recurrence_interval IS NOT NULL AND recurrence_end_date IS NOT NULL)
    )
);

CREATE INDEX idx_events_subject ON events (subject);
CREATE INDEX idx_events_starts_at ON events (starts_at);

CREATE TABLE event_groups (
    event_id  UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    subject   TEXT NOT NULL,
    grp       TEXT NOT NULL,
    PRIMARY KEY (event_id, subject, grp),
    FOREIGN KEY (subject, grp) REFERENCES subject_groups (subject, grp)
);

CREATE INDEX idx_event_groups_subject_grp ON event_groups (subject, grp);
