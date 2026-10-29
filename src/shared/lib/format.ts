/** Formatage des nombres et durées pour l'affichage (locale de l'interface). */

/** Nombre formaté selon la langue, avec exactement `digits` décimales ; vide → 0. */
export function formatNumber(value: number | null | undefined, language: string, digits = 0) {
  return new Intl.NumberFormat(language, {
    maximumFractionDigits: digits,
    minimumFractionDigits: digits,
  }).format(value ?? 0);
}

/** « 1:05 » pour 65 s ; avec `padMinutes`, « 01:05 » (chronomètre du bandeau). */
export function formatDuration(ms: number, padMinutes = false): string {
  // Secondes entières, puis découpage en minutes et secondes sur deux chiffres.
  const total = Math.floor(ms / 1000);
  const minutes = Math.floor(total / 60).toString();
  const seconds = total % 60;
  return `${padMinutes ? minutes.padStart(2, '0') : minutes}:${seconds.toString().padStart(2, '0')}`;
}

/** Durée longue : « 31 h 40 », « 2 h 05 », ou « 45 min » sous une heure. */
export function formatHours(ms: number): string {
  const minutes = Math.floor(ms / 60_000);
  const hours = Math.floor(minutes / 60);
  // Sous une heure, les minutes seules suffisent.
  if (hours === 0) return `${minutes} min`;
  return `${hours} h ${(minutes % 60).toString().padStart(2, '0')}`;
}

/** Écart signé : « +3 », « −0,8 », « ±0 » (signe moins typographique). */
export function formatDelta(value: number, language: string, digits = 0): string {
  const sign = value > 0 ? '+' : value < 0 ? '−' : '±';
  return `${sign}${formatNumber(Math.abs(value), language, digits)}`;
}
