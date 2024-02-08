CREATE TYPE category AS ENUM (
    'article',
    'email',
    'epub',
    'highlight',
    'note',
    'pdf',
    'rss',
    'tweet',
    'video'
);

CREATE TYPE location AS ENUM (
    'archive',
    'feed',
    'later',
    'new',
    'shortlist'
);

CREATE TABLE reading(
  id                TEXT PRIMARY KEY,
  author            TEXT,
  category          category,
  content           TEXT,
  created_at        TIMESTAMP WITH TIME ZONE NOT NULL,
  image_url         TEXT,
  location          location,
  notes             TEXT,
  parent_id         TEXT,
  published_date    TIMESTAMP WITH TIME ZONE,
  reading_progress  REAL,
  readwise_url      TEXT,
  site_name         TEXT,
  source            TEXT,
  source_url        TEXT,
  summary           TEXT,
  tags              JSONB,
  title             TEXT NOT NULL,
  updated_at        TIMESTAMP WITH TIME ZONE,
  word_count        SERIAL
);

CREATE UNIQUE INDEX source_url_idx ON reading (source_url);
