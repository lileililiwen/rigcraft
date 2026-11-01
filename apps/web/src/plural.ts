/**
 * One pluralisation helper for every count a page composes. It never produces
 * an `(s)` pseudo-plural, so a count of one and a count of many read correctly.
 */
export function pluralWord(
  count: number,
  singular: string,
  pluralForm?: string,
): string {
  return count === 1 ? singular : (pluralForm ?? `${singular}s`);
}

export function plural(count: number, singular: string, pluralForm?: string): string {
  return `${count} ${pluralWord(count, singular, pluralForm)}`;
}
