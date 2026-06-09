CREATE TYPE auto_select_job_status AS ENUM ('processing', 'completed', 'failed');

CREATE TABLE auto_select_jobs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  status auto_select_job_status NOT NULL DEFAULT 'processing',
  groups_selected INT,
  error TEXT,
  started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  finished_at TIMESTAMPTZ
);

CREATE INDEX ON auto_select_jobs (user_id);

-- Evitar que un usuario pueda lanzar dos jobs
CREATE UNIQUE INDEX autoselect_jobs_user_processing
  ON auto_select_jobs (user_id)
  WHERE status = 'processing';
