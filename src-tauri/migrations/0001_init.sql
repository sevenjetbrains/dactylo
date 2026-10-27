-- Schéma initial (cahier des charges, section 3.3).
-- Identifiants : UUIDv7 en TEXT. Dates : millisecondes Unix en INTEGER (ADR 0003).
-- Ne jamais modifier ce fichier : toute évolution = nouvelle migration (ADR 0004).

CREATE TABLE profiles (
  id              TEXT PRIMARY KEY,
  name            TEXT NOT NULL,
  keyboard_layout TEXT NOT NULL DEFAULT 'azerty'
                  CHECK (keyboard_layout IN ('azerty', 'qwerty', 'bepo')),
  settings        TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(settings)),
  created_at      INTEGER NOT NULL
);

-- Bibliothèque partagée entre les profils ; la progression reste par profil.
CREATE TABLE texts (
  id          TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  content     TEXT NOT NULL,
  language    TEXT NOT NULL DEFAULT 'fr',
  source      TEXT NOT NULL CHECK (source IN ('paste', 'pdf', 'builtin')),
  source_file TEXT,
  pages       TEXT,                                    -- ex. "3-5"
  tags        TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(tags)),
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

CREATE TABLE text_progress (
  profile_id     TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  text_id        TEXT NOT NULL REFERENCES texts(id) ON DELETE CASCADE,
  position       INTEGER NOT NULL DEFAULT 0,
  last_opened_at INTEGER NOT NULL,
  PRIMARY KEY (profile_id, text_id)
) WITHOUT ROWID;

CREATE TABLE sessions (
  id          TEXT PRIMARY KEY,
  profile_id  TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  text_id     TEXT REFERENCES texts(id) ON DELETE SET NULL,
  mode        TEXT NOT NULL CHECK (mode IN ('copy', 'dictation', 'review')),
  started_at  INTEGER NOT NULL,
  ended_at    INTEGER,                                 -- NULL = en cours, reprise possible (NF-06)
  duration_ms INTEGER,
  wpm_raw     REAL,
  wpm_net     REAL,
  cpm         REAL,
  accuracy    REAL,
  settings    TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(settings))
);
CREATE INDEX idx_sessions_profile_started ON sessions(profile_id, started_at);

-- Données brutes : chaque frappe, y compris les retours arrière (F-31).
CREATE TABLE keystrokes (
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  seq        INTEGER NOT NULL,                          -- ordre de frappe
  position   INTEGER NOT NULL,                          -- index du graphème dans le texte
  kind       TEXT NOT NULL CHECK (kind IN ('insert', 'delete')),
  expected   TEXT,                                      -- graphème NFC attendu
  typed      TEXT,                                      -- graphème NFC tapé
  t_ms       INTEGER NOT NULL,                          -- depuis le début de session
  correct    INTEGER NOT NULL CHECK (correct IN (0, 1)),
  PRIMARY KEY (session_id, seq)
) WITHOUT ROWID;

CREATE TABLE session_errors (
  id            INTEGER PRIMARY KEY,
  session_id    TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  expected_word TEXT,
  typed_word    TEXT,
  error_type    TEXT NOT NULL CHECK (error_type IN (
                  'accent', 'missing_letter', 'extra_letter', 'transposition',
                  'substitution', 'missing_word', 'extra_word', 'punctuation', 'case')),
  position      INTEGER NOT NULL
);
CREATE INDEX idx_session_errors_session ON session_errors(session_id);

CREATE TABLE words (
  id             TEXT PRIMARY KEY,
  profile_id     TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  word           TEXT NOT NULL,                         -- forme normalisée (NFC)
  encounters     INTEGER NOT NULL DEFAULT 0,
  errors         INTEGER NOT NULL DEFAULT 0,
  avg_ms         REAL,
  mastery        TEXT NOT NULL DEFAULT 'learning'
                 CHECK (mastery IN ('learning', 'mastered', 'manual_mastered')),
  streak         INTEGER NOT NULL DEFAULT 0,            -- réussites consécutives (F-42)
  last_error_at  INTEGER,
  next_review_at INTEGER,
  srs_state      TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(srs_state)),
  UNIQUE (profile_id, word)
);
CREATE INDEX idx_words_review ON words(profile_id, next_review_at);

CREATE TABLE word_attempts (
  id           INTEGER PRIMARY KEY,
  word_id      TEXT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
  session_id   TEXT REFERENCES sessions(id) ON DELETE SET NULL,
  typed_form   TEXT NOT NULL,
  correct      INTEGER NOT NULL CHECK (correct IN (0, 1)),
  duration_ms  INTEGER,
  attempted_at INTEGER NOT NULL
);
CREATE INDEX idx_word_attempts_word ON word_attempts(word_id);

CREATE TABLE dictation_profiles (
  id               TEXT PRIMARY KEY,
  profile_id       TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  name             TEXT NOT NULL,
  voice            TEXT,
  rate             REAL NOT NULL DEFAULT 1.0,
  pitch            REAL NOT NULL DEFAULT 1.0,
  volume           REAL NOT NULL DEFAULT 1.0,
  segmentation     TEXT NOT NULL DEFAULT 'group'
                   CHECK (segmentation IN ('word', 'group', 'sentence', 'paragraph')),
  group_size       INTEGER NOT NULL DEFAULT 4,
  pause_ms         INTEGER NOT NULL DEFAULT 1000,
  read_punctuation INTEGER NOT NULL DEFAULT 0 CHECK (read_punctuation IN (0, 1))
);
