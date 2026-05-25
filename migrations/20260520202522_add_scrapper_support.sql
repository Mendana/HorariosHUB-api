-- 1. Campos para sesiones que desaparecen del scraper
ALTER TABLE sessions
    ADD COLUMN scraper_missing       BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN scraper_missing_since TIMESTAMPTZ;

-- 2. Lock de ejecución — evita scrapers simultáneos
CREATE TABLE scraper_locks (
    id        TEXT PRIMARY KEY,
    locked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_by TEXT NOT NULL
);

-- 3. Histórico de cambios archivados
CREATE TABLE changes_history (
    id             UUID PRIMARY KEY,
    proposed_by    UUID NOT NULL REFERENCES users(id),
    session_id     UUID, 
    subject        TEXT,
    grp            TEXT,
    change_type    change_type NOT NULL,
    change_status  change_status NOT NULL,
    prev_starts_at TIMESTAMPTZ,
    prev_duration  INTEGER,
    new_starts_at  TIMESTAMPTZ,
    new_duration   INTEGER,
    proposed_at    TIMESTAMPTZ NOT NULL,
    prev_classroom TEXT,
    new_classroom  TEXT,

    archived_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_changes_history_proposed_by
    ON changes_history (proposed_by);
CREATE INDEX idx_changes_history_archived_at
    ON changes_history (archived_at DESC);
CREATE INDEX idx_changes_history_status
    ON changes_history (change_status);
