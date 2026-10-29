/** Sons de frappe (F-51) : petit clic à chaque frappe juste, son grave sur une
 * erreur. Synthétisés avec WebAudio : aucun fichier audio, rien à charger (NF-04). */

/** Contexte audio, créé à la première frappe (les navigateurs l'exigent). */
let context: AudioContext | null = null;

/** Contexte audio, ou `null` si WebAudio est indisponible (tests). */
function audio(): AudioContext | null {
  if (context) return context;
  const Ctor = (globalThis as { AudioContext?: typeof AudioContext }).AudioContext;
  if (!Ctor) return null;
  context = new Ctor();
  return context;
}

/** Joue un son bref : fréquence, forme, durée et volume. */
function beep(frequency: number, type: OscillatorType, seconds: number, volume: number) {
  const ctx = audio();
  if (!ctx) return;
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = type;
  osc.frequency.value = frequency;
  // Attaque immédiate, extinction rapide : un « clic » plutôt qu'une note.
  gain.gain.setValueAtTime(volume, ctx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + seconds);
  osc.connect(gain).connect(ctx.destination);
  osc.start();
  osc.stop(ctx.currentTime + seconds);
}

/** Son d'une frappe : clic léger si elle est juste, son grave sinon. */
export function playKeySound(correct: boolean): void {
  if (correct) beep(1800, 'triangle', 0.03, 0.08);
  else beep(160, 'square', 0.12, 0.06);
}
