/** Formatage des nombres et durées pour l'affichage (locale de l'interface). */

/** Nombre formaté selon la langue, avec exactement `digits` décimales ; vide → 0. */
export function formatNumber(value: number | null | undefined, language: string, digits = 0) {
  return new Intl.NumberFormat(language, {
    maximumFractionDigits: digits,
    minimumFractionDigits: digits,
  }).format(value ?? 0);
}

/** « 1:05 » pour 65 s. */
export function formatDuration(ms: number): string {
  // Secondes entières, puis découpage en minutes et secondes sur deux chiffres.
  const total = Math.floor(ms / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}
