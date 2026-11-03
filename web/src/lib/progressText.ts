// The words and amounts of a loading bar, in the active locale. Apart from
// progress.ts, which the decoding worker runs and must not depend on the
// interface's language.
import { formatNumber, withUnit } from './format';
import { t } from './i18n/index.svelte';
import type { LoadProgress } from './progress';

/** "12.3 / 45.6 MB", "12.3 MB" while the size is unknown, or "2 / 5" for steps. */
export function progressText(progress: LoadProgress): string {
  if (progress.stage === 'decode') {
    return progress.total === null ? '' : `${progress.done} / ${progress.total}`;
  }
  const mb = (bytes: number) => formatNumber(bytes / 1e6, 1);
  return withUnit(progress.total === null ? mb(progress.done) : `${mb(progress.done)} / ${mb(progress.total)}`, 'MB');
}

/** What the bar says is being worked on, in the active locale. */
export function progressLabel(progress: LoadProgress): string {
  if (progress.stage === 'download') {
    return progress.label;
  }
  return progress.label ? t.loading.decoding({ file: progress.label }) : t.loading.stats;
}
