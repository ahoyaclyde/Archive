-- =============================================================================
-- MIGRATION: Face Encoding Support
-- Version  : 001
-- Date     : 2026-03-02
-- Purpose  : Adds face_encodings table + phash/auto_generated columns to
--            targets.  Safe to run against an existing production database —
--            every statement is idempotent (IF NOT EXISTS / IF NOT EXISTS).
-- Usage    : sqlite3 your_database.db < migrate_face_encodings.sql
-- =============================================================================

PRAGMA foreign_keys = OFF;   -- disable FK checks during migration
BEGIN TRANSACTION;

-- -----------------------------------------------------------------------------
-- 1. Add new columns to the targets table
--    SQLite does not support ADD COLUMN IF NOT EXISTS, so we use a
--    try-and-ignore pattern: each ALTER TABLE is wrapped so that if the column
--    already exists (e.g. you run the migration twice) it fails silently inside
--    the same transaction and we continue.
--
--    In practice you can run this with the sqlite3 CLI and it will print:
--      "duplicate column name: phash"  → already applied, safe to ignore.
-- -----------------------------------------------------------------------------

-- phash: 16-char hex perceptual hash computed by the JS face sidecar.
-- Used as a fallback matcher when no face is detected (vehicles, objects, etc.)
ALTER TABLE targets ADD COLUMN phash TEXT;

-- auto_generated: 1 = this target profile was created automatically because
-- a face was detected in the image but the uploader did not manually select it.
-- 0 = user-selected target (default, backwards-compatible).
ALTER TABLE targets ADD COLUMN auto_generated INTEGER NOT NULL DEFAULT 0;

-- -----------------------------------------------------------------------------
-- 2. Add index on targets.phash so pHash fallback lookups are fast
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_targets_phash ON targets(phash);

-- -----------------------------------------------------------------------------
-- 3. Create the face_encodings table
--    Stores one row per detected face per target image.
--    descriptor BLOB = 128 × f32 little-endian = 512 bytes per face.
--    Matching (Euclidean distance) is computed in Rust — SQLite holds storage.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS face_encodings (
    id              TEXT    PRIMARY KEY,
    target_id       TEXT    NOT NULL,
    evidence_id     TEXT    NOT NULL,

    -- Which face within the image (0 = first/largest, 1 = second, etc.)
    face_index      INTEGER NOT NULL DEFAULT 0,

    -- 128 × f32 serialised little-endian = 512 bytes
    descriptor      BLOB    NOT NULL,

    -- face-api.js detection confidence score (0.0 – 1.0)
    detection_score REAL    NOT NULL DEFAULT 0.0,

    -- pHash of the full image (fallback for non-face targets, nullable)
    phash           TEXT,

    -- 1 = auto-created from multi-face split, not user-confirmed
    auto_generated  INTEGER NOT NULL DEFAULT 0,

    created_at      INTEGER NOT NULL,

    FOREIGN KEY (target_id)   REFERENCES targets(id)  ON DELETE CASCADE,
    FOREIGN KEY (evidence_id) REFERENCES evidence(id) ON DELETE CASCADE
);

-- -----------------------------------------------------------------------------
-- 4. Indexes on face_encodings
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_face_encodings_target
    ON face_encodings(target_id);

CREATE INDEX IF NOT EXISTS idx_face_encodings_evidence
    ON face_encodings(evidence_id);

-- Lets you quickly count / filter auto-generated targets
CREATE INDEX IF NOT EXISTS idx_face_encodings_auto
    ON face_encodings(auto_generated);

-- Composite: handy when loading all faces for a specific target in order
CREATE INDEX IF NOT EXISTS idx_face_encodings_target_face
    ON face_encodings(target_id, face_index);

-- -----------------------------------------------------------------------------
-- 5. Record this migration in a schema_migrations table so you have an audit
--    trail of which migrations have been applied.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     TEXT    PRIMARY KEY,
    description TEXT    NOT NULL,
    applied_at  INTEGER NOT NULL
);

INSERT OR IGNORE INTO schema_migrations (version, description, applied_at)
VALUES (
    '001_face_encodings',
    'Add face_encodings table + phash/auto_generated to targets',
    strftime('%s', 'now')
);

-- -----------------------------------------------------------------------------
-- 6. Verify
-- -----------------------------------------------------------------------------
SELECT
    'Migration 001 complete' AS status,
    (SELECT COUNT(*) FROM face_encodings) AS face_encodings_rows,
    (SELECT COUNT(*) FROM schema_migrations WHERE version = '001_face_encodings') AS migration_recorded;

COMMIT;
PRAGMA foreign_keys = ON;