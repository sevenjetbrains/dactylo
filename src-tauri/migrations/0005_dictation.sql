-- Mode Dictée (F-10 à F-17, ADR 0020).
-- Ne jamais modifier ce fichier : toute évolution = nouvelle migration (ADR 0004).

-- Texte tapé pendant une dictée : brouillon toutes les 10 s (NF-06), puis copie finale.
ALTER TABLE sessions ADD COLUMN typed_text TEXT;

-- Réglages des profils de dictée absents du schéma initial (F-14, F-15).
ALTER TABLE dictation_profiles ADD COLUMN advance TEXT NOT NULL DEFAULT 'adaptive'
  CHECK (advance IN ('adaptive', 'automatic'));
ALTER TABLE dictation_profiles ADD COLUMN max_repeats INTEGER;
ALTER TABLE dictation_profiles ADD COLUMN lenient_punctuation INTEGER NOT NULL DEFAULT 1
  CHECK (lenient_punctuation IN (0, 1));

-- Profils de dictée d'un profil utilisateur, par nom.
CREATE INDEX idx_dictation_profiles_profile ON dictation_profiles(profile_id, name);
