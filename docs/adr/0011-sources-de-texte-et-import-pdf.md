# 0011 — Sources de texte et import PDF

- Statut : accepté — 2026-10-28

## Décision

- Toute provenance de texte implémente le trait `TextSource` du cœur (`dactylo-core::source`). Une source produit un **brouillon** (`TextDraft` : contenu normalisé, provenance, fichier, pages) ou une `SourceError` (texte vide, PDF scanné, plage impossible). Trois sources existent : `PasteSource` (F-20), `PdfSource` (F-21) et `BuiltinSource` (F-25). OCR, EPUB ou site web s'ajouteront de la même façon.
- **Partage du travail pour un PDF** : pdf.js (`pdfjs-dist`, côté interface) ouvre le fichier, affiche l'aperçu des pages et extrait les lignes de chaque page. Tout le reste est de la logique métier, dans le cœur :
  - choix de la plage de pages et des lignes ;
  - suppression des numéros de page et des en-têtes ou pieds de page répétés ;
  - recollage des césures et reconstitution des paragraphes ;
  - normalisation des espaces et des apostrophes.

  La commande `preview_pdf` ne stocke rien. Elle renvoie les lignes numérotées et le texte nettoyé.

- L'interface envoie les pages de la plage **et jusqu'à deux pages voisines de chaque côté**. Un en-tête de page paire n'apparaît qu'une fois sur les pages 3 à 5 : les voisines permettent de le reconnaître. Est retenue comme en-tête une première ou dernière ligne qui se répète, chiffres ignorés, sur au moins deux pages et sur au moins la moitié des pages fournies.
- Un numéro de page fait au plus trois chiffres : une ligne « 1872 » est plus probablement une année.
- Le texte nettoyé est modifiable avant l'enregistrement (F-22). La commande `import_pdf` reçoit le texte final, qu'elle normalise de nouveau, ainsi que le fichier et la plage, rangés dans `texts.source_file` et `texts.pages`.
- Chaque étape du nettoyage peut être désactivée. Exemple : la reconstitution des paragraphes, inutile pour un poème.

## Conséquences

Le nettoyage est testé en Rust, sans navigateur. Les e2e vérifient l'extraction réelle par pdf.js sur `tests/e2e/fixtures/livre.pdf`, généré par `make-pdf.py`. Les heuristiques peuvent se tromper, par exemple sur un mot composé coupé en fin de ligne (« peut- / être ») : l'aperçu modifiable sert de filet. Un PDF scanné donne un message clair ; l'OCR viendra plus tard comme une nouvelle `TextSource`.
