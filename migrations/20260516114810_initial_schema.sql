CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Enums
CREATE TYPE user_role     AS ENUM ('admin', 'professor', 'student');
CREATE TYPE change_type   AS ENUM ('create', 'modify', 'delete');
CREATE TYPE change_status AS ENUM ('pending', 'approved', 'rejected');

-- Usuarios
CREATE TABLE users (
    id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email     TEXT NOT NULL UNIQUE,
    role      user_role NOT NULL,
    verified  BOOLEAN NOT NULL DEFAULT false
);

-- Asignaturas y grupos
CREATE TABLE subject_groups (
    subject       TEXT NOT NULL,
    grp           TEXT NOT NULL,
    PRIMARY KEY (subject, grp)
);

-- Sesiones concretas
CREATE TABLE sessions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject       TEXT NOT NULL,
    grp           TEXT NOT NULL,
    starts_at     TIMESTAMPTZ NOT NULL,
    duration_min  INTEGER NOT NULL,
    is_overridden BOOLEAN NOT NULL DEFAULT false,
    scraped_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (subject, grp) REFERENCES subject_groups(subject, grp),
    UNIQUE (subject, grp, starts_at)
);

-- Suscripciones de alumnos a grupos
CREATE TABLE schedule (
    user_id  UUID NOT NULL REFERENCES users(id),
    subject  TEXT NOT NULL,
    grp      TEXT NOT NULL,
    PRIMARY KEY (user_id, subject, grp),
    FOREIGN KEY (subject, grp) REFERENCES subject_groups(subject, grp)
);

-- Propuestas de cambio
CREATE TABLE changes (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposed_by   UUID NOT NULL REFERENCES users(id),
    session_id    UUID REFERENCES sessions(id),
    subject       TEXT,
    grp           TEXT,
    change_type   change_type NOT NULL,
    change_status change_status NOT NULL DEFAULT 'pending',
    prev_starts_at  TIMESTAMPTZ,
    prev_duration   INTEGER,
    new_starts_at   TIMESTAMPTZ,
    new_duration    INTEGER,
    proposed_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (subject, grp) REFERENCES subject_groups(subject, grp),
    CONSTRAINT chk_create CHECK (change_type != 'create' OR (session_id IS NULL     AND subject IS NOT NULL AND grp IS NOT NULL)),
    CONSTRAINT chk_modify CHECK (change_type != 'modify' OR (session_id IS NOT NULL AND subject IS NULL     AND grp IS NULL)),
    CONSTRAINT chk_delete CHECK (change_type != 'delete' OR (session_id IS NOT NULL AND subject IS NULL     AND grp IS NULL))
);

-- Índices
CREATE INDEX idx_sessions_starts_at
    ON sessions (starts_at);

CREATE INDEX idx_sessions_overridden
    ON sessions (id)
    WHERE is_overridden = true;

CREATE INDEX idx_changes_status_proposed_at
    ON changes (change_status, proposed_at DESC);

CREATE INDEX idx_changes_session_id
    ON changes (session_id);
