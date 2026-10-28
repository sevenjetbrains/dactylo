# 0008 — Comparaison de frappe en miroir (TS pour l'affichage, Rust fait foi)

- Statut : accepté — 2026-10-28

## Contexte

La règle du projet place la logique métier dans `crates/core`. Or l'affichage de chaque frappe doit prendre moins de 16 ms (NF-01, F-02), et le mode bloquant doit décider de la position du curseur avant de rendre la main. Un aller-retour IPC par touche ajoute une latence variable ; compiler le cœur en WebAssembly alourdirait la chaîne de build.

## Décision

- `dactylo-core::typing::TypingSession` est la **référence** : il rejoue les frappes brutes (`insert` + graphème, ou `delete`) et décide seul de leur position, du caractère attendu et de leur justesse. Les statistiques, les fautes et la reprise (NF-06) sont calculées à partir de ces frappes.
- L'interface envoie uniquement des frappes brutes (`seq`, nature, graphème, temps actif), par lots toutes les 2 s. Elle ne transmet jamais sa propre décision « juste / faux ».
- `src/modules/practice/engine/` contient un **miroir** TypeScript de `TypingSession`, réservé à l'affichage immédiat et au bandeau en direct (F-05). Il ne produit aucune donnée enregistrée.
- Les deux implémentations jouent les mêmes cas : `crates/core/tests/fixtures/typing-cases.json` (positions, états des cellules, compteurs). Toute évolution des règles commence par un cas dans ce fichier.
- Le texte est découpé en graphèmes par le cœur et transmis tel quel à l'interface : les deux côtés partagent donc exactement les mêmes positions.

## Conséquences

- Une divergence entre miroir et cœur ne peut fausser que l'affichage, jamais les données ; les cas partagés la détectent en CI.
- Le miroir doit rester petit : pas de statistiques finales ni de classement des fautes côté TS.
