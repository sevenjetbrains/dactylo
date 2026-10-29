/** Dates locales : fuseau transmis au backend (ADR 0017) et libellés relatifs. */

/** Une journée, en millisecondes. */
const DAY_MS = 86_400_000;

/** Décalage du fuseau local par rapport à UTC, en minutes (positif à l'est). */
export function tzOffsetMinutes(at: number = Date.now()): number {
  // `getTimezoneOffset` compte dans l'autre sens (−60 pour Paris en hiver).
  return -new Date(at).getTimezoneOffset();
}

/** Minuit local du jour qui contient `ms`. */
function startOfDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** Nombre de jours locaux entre aujourd'hui et `ms` : 0 aujourd'hui, 1 demain, −1 hier. */
export function dayDiff(ms: number, now: number = Date.now()): number {
  // Arrondi : un changement d'heure rend une journée longue de 23 ou 25 h.
  return Math.round((startOfDay(ms) - startOfDay(now)) / DAY_MS);
}

/** Met la première lettre en majuscule (« aujourd’hui » → « Aujourd’hui »). */
function capitalize(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** Jour relatif : « Aujourd’hui », « Demain », « Hier », « Dans 4 jours »… */
export function relativeDay(ms: number, language: string, now: number = Date.now()): string {
  const format = new Intl.RelativeTimeFormat(language, { numeric: 'auto' });
  return capitalize(format.format(dayDiff(ms, now), 'day'));
}

/** Date courte : « 1 oct. ». */
export function shortDate(ms: number, language: string): string {
  return new Intl.DateTimeFormat(language, { day: 'numeric', month: 'short' }).format(ms);
}

/** Heure : « 18:42 ». */
export function timeOfDay(ms: number, language: string): string {
  return new Intl.DateTimeFormat(language, { hour: '2-digit', minute: '2-digit' }).format(ms);
}

/**
 * Date d'une session : « Aujourd’hui, 18:42 », « Hier, 21:10 », sinon « 24 oct., 9:05 ».
 */
export function sessionDate(ms: number, language: string, now: number = Date.now()): string {
  const diff = dayDiff(ms, now);
  // Aujourd'hui et hier en toutes lettres, au-delà la date courte.
  const day = diff === 0 || diff === -1 ? relativeDay(ms, language, now) : shortDate(ms, language);
  return `${day}, ${timeOfDay(ms, language)}`;
}
