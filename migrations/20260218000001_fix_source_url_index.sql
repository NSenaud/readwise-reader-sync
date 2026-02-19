-- Drop the unique constraint: multiple documents (e.g. highlights, notes)
-- can legitimately share the same source_url. The primary key on `id` is
-- sufficient for uniqueness.
DROP INDEX IF EXISTS source_url_idx;

-- Re-add as a non-unique partial index (excludes NULLs, which are unindexable
-- for equality lookups and make up a significant portion of the column).
CREATE INDEX source_url_idx ON reading (source_url) WHERE source_url IS NOT NULL;
