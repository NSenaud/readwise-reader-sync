-- word_count was incorrectly defined as SERIAL (auto-increment sequence).
-- Change to INTEGER so the application-provided value is stored correctly.
ALTER TABLE reading ALTER COLUMN word_count DROP DEFAULT;
ALTER TABLE reading ALTER COLUMN word_count TYPE INTEGER;
ALTER TABLE reading ALTER COLUMN word_count SET DEFAULT 0;
DROP SEQUENCE IF EXISTS reading_word_count_seq;

-- Enforce that reading_progress stays within the [0.0, 1.0] domain.
ALTER TABLE reading ADD CONSTRAINT reading_progress_bounds
    CHECK (reading_progress >= 0.0 AND reading_progress <= 1.0);
