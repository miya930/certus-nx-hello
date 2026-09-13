// 色は styles.css の --series-1 … --series-16 に定義し、ライトとダークで切り替える。
export const SERIES_COLORS = 16;

export interface SeriesStyle {
  label: string;
  color: string;
}

export function seriesStyle(index: number): SeriesStyle {
  return { label: `H${index + 1}`, color: `var(--series-${(index % SERIES_COLORS) + 1})` };
}
