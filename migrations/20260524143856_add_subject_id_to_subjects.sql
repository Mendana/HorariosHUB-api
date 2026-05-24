ALTER TABLE subject_groups
  ADD COLUMN id UUID NOT NULL DEFAULT gen_random_uuid();
