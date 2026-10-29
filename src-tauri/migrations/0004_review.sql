-- Suivi des mots difficiles (F-40 à F-45, ADR 0016) et statistiques (ADR 0017).
-- Ne jamais modifier ce fichier : toute évolution = nouvelle migration (ADR 0004).

-- Type de faute d'une tentative ratée, pour la « faute dominante » d'un mot (F-41).
ALTER TABLE word_attempts ADD COLUMN error_type TEXT CHECK (error_type IS NULL OR error_type IN (
  'accent', 'missing_letter', 'extra_letter', 'transposition',
  'substitution', 'missing_word', 'extra_word', 'punctuation', 'case'));

-- Tentatives d'un mot par date : formes tapées récentes et dernière faute.
DROP INDEX idx_word_attempts_word;
CREATE INDEX idx_word_attempts_word_time ON word_attempts(word_id, attempted_at);

-- Tentatives d'une session (suppression en cascade, analyses par session).
CREATE INDEX idx_word_attempts_session ON word_attempts(session_id);

-- Mots en révision d'un profil, par niveau de maîtrise.
CREATE INDEX idx_words_mastery ON words(profile_id, mastery);
