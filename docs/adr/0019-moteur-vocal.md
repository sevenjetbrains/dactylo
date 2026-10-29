# 0019 — Moteur vocal de la dictée

- Statut : accepté — 2026-10-29

## Contexte

La dictée (F-10 à F-12) lit le texte à voix haute, hors ligne (NF-03). La pile retenue impose la crate `tts`, derrière un trait `SpeechEngine`, pour accueillir plus tard des voix neuronales hors ligne (Piper).

## Décision

- `dactylo_core::speech::SpeechEngine` : `voices()`, `speak(texte, réglages) -> numéro de lecture`, `stop()`. Les réglages sont relatifs : vitesse et hauteur où 1 vaut la normale, volume de 0 à 1. `EngineRange` les convertit vers les bornes propres à chaque moteur. `pick_voice` prend la voix demandée, sinon la première voix française.
- `TtsEngine` (src-tauri) s'appuie sur la crate `tts`. Sous Windows, elle passe par WinRT et donne les voix locales (Hortense, Julie, Paul…). Les voix neuronales en ligne, comme « Denise » sur la maquette, ne sont pas disponibles.
  - Le type `tts::Tts` repose sur `Rc` : il est confié à un **fil dédié**, piloté par un canal.
- **Fin de lecture** : le rappel `on_utterance_end` déclenche l'événement Tauri `SpeechFinished { utterance }`. L'interface ignore les fins d'anciennes lectures (segment répété, retour en arrière).
- Sans synthèse disponible, `UnavailableEngine` refuse chaque lecture avec un message lisible. Les tests utilisent `FakeSpeechEngine`, qui mémorise les textes lus, sans son.
- Un test ignoré par défaut (`cargo test -- --ignored`) vérifie le vrai moteur : voix françaises, lecture muette, rappel de fin.

## Conséquences

- Aucune permission Tauri supplémentaire ni accès réseau (NF-11).
- La qualité des voix dépend du système. Piper pourra devenir une seconde implémentation de `SpeechEngine`, choisie dans les réglages.
