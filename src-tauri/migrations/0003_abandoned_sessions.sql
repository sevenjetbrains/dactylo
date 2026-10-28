-- Sessions abandonnées conservées avec leurs frappes (F-31, ADR 0014).
-- Ne jamais modifier ce fichier : toute évolution = nouvelle migration (ADR 0004).

-- État d'une session : en cours (ni fin ni abandon), terminée (`ended_at`)
-- ou abandonnée (`abandoned_at`, avec `duration_ms` = temps actif de la dernière frappe).
ALTER TABLE sessions ADD COLUMN abandoned_at INTEGER;
