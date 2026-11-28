-- Community Builds (D1). Apply with: npx wrangler d1 execute babel-gallery --remote --file schema.sql
-- Times are milliseconds since 1970 (UTC).

CREATE TABLE IF NOT EXISTS builds (
  id          TEXT PRIMARY KEY,              -- 10 letters and digits, in the build's link
  kind        TEXT NOT NULL,                 -- 'player' | 'demon'
  secret_hash TEXT NOT NULL,                 -- SHA-256 of the secret BABEL keeps (only it can change or delete)
  name        TEXT NOT NULL,                 -- the display name the player typed
  title       TEXT NOT NULL,
  level       INTEGER NOT NULL,              -- the character's, or the demon's
  created     INTEGER NOT NULL,
  updated     INTEGER NOT NULL,
  hidden      INTEGER NOT NULL DEFAULT 0,    -- 1: off the lists (reported, or hidden by the developer)
  checked     INTEGER NOT NULL DEFAULT 0,    -- reports already seen by the developer
  facets      TEXT NOT NULL,                 -- JSON: what the lists filter on (src/build.js facets)
  body        TEXT NOT NULL                  -- JSON: the build as check() cleaned it
);
CREATE INDEX IF NOT EXISTS builds_list ON builds (kind, hidden, updated);

-- New builds per address and day (who = keyed hash of the address; rows older than a day are deleted).
CREATE TABLE IF NOT EXISTS creates (
  who TEXT NOT NULL,
  at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS creates_who ON creates (who, at);

-- One report per person and build.
CREATE TABLE IF NOT EXISTS reports (
  build  TEXT NOT NULL,
  who    TEXT NOT NULL,
  reason TEXT NOT NULL,
  at     INTEGER NOT NULL,
  PRIMARY KEY (build, who)
);
