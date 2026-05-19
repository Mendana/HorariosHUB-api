CREATE TYPE session_source AS ENUM ('scraper', 'manual');

ALTER TABLE sessions
    ADD COLUMN classroom  TEXT,
    ADD COLUMN source     session_source NOT NULL DEFAULT 'scraper',
    ADD COLUMN created_by UUID REFERENCES users(id);

-- Sesiones manuales siempre tienen autor
ALTER TABLE sessions ADD CONSTRAINT chk_manual_has_creator
    CHECK (source != 'manual' OR created_by IS NOT NULL);

-- scraped_at pasa a nullable — NULL en sesiones manuales
ALTER TABLE sessions ALTER COLUMN scraped_at DROP NOT NULL;
