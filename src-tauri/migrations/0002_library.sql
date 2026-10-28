-- Phase 2 : bibliothèque et import PDF (F-21 à F-25).
-- Ne jamais modifier ce fichier : toute évolution = nouvelle migration (ADR 0004).

-- Découpage en exercices (F-24, ADR 0012) : JSON de `Segmentation` côté cœur.
ALTER TABLE texts ADD COLUMN segmentation TEXT NOT NULL DEFAULT '{"type":"whole"}'
  CHECK (json_valid(segmentation));

-- Auteur, surtout pour le pack intégré (F-25) : « Jules Verne ».
ALTER TABLE texts ADD COLUMN author TEXT;

-- Valeurs propres à l'installation, ex. version du pack intégré déjà inséré (ADR 0013).
CREATE TABLE app_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) WITHOUT ROWID;
