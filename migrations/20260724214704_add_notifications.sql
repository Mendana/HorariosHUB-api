CREATE TYPE notification_type AS ENUM (
  'session_modified',
  'session_deleted',
  'exam_added',
  'proposal_approved',
  'proposal_rejected',
  'proposal_created',
  'scrapper_conflict'
);

CREATE TABLE notifications (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  type notification_type NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  session_id UUID REFERENCES sessions(id) ON DELETE SET NULL,
  proposal_id UUID REFERENCES changes(id) ON DELETE SET NULL,
  read BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_notifications_user_id_read_created_at
  ON notifications (user_id, read, created_at DESC);

ALTER TABLE users
  ADD COLUMN notify_in_app BOOLEAN NOT NULL DEFAULT true,
  ADD COLUMN notify_email BOOLEAN NOT NULL DEFAULT false;
