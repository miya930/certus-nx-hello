import { useMemo } from "react";

export interface MiniSeries {
  label: string;
  values: number[];
  color: string;
  dash?: string;
  /** 凡例に出さない補助線。 */
  quiet?: boolean;
}

interface Props {
  title: string;
  unit: string;
  times: number[];
  series: MiniSeries[];
  height?: number;
  includeZero?: boolean;
  /** 太い縦線を引く時刻。デモで選んだ時刻を示す。 */
  marker?: number;
}

const WIDTH = 640;
const MARGIN = { top: 10, right: 12, bottom: 24, left: 44 };
const Y_TICKS = 4;
const X_TICKS = 6;
const MAX_POINTS = 320;

function niceStep(span: number, count: number) {
  const raw = span / count;
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const normalized = raw / magnitude;
  return (normalized < 1.5 ? 1 : normalized < 3 ? 2 : normalized < 7 ? 5 : 10) * magnitude;
}

function ticks(min: number, max: number, count: number) {
  const step = niceStep(max - min || 1, count);
  const out: number[] = [];
  for (let v = Math.ceil(min / step) * step; v <= max + step * 1e-6; v += step) out.push(Number(v.toFixed(10)));
  return out;
}

/** 軸と凡例だけの軽いグラフ。Notes のデモで、スライダーを動かすたびに描き直す。 */
export function MiniChart({ title, unit, times, series, height = 180, includeZero = false, marker }: Props) {
  const plotWidth = WIDTH - MARGIN.left - MARGIN.right;
  const plotHeight = height - MARGIN.top - MARGIN.bottom;
  const duration = times[times.length - 1] ?? 1;
  const stride = Math.max(1, Math.ceil(times.length / MAX_POINTS));

  const [yMin, yMax] = useMemo(() => {
    let low = includeZero ? 0 : Infinity;
    let high = -Infinity;
    for (const s of series) for (const v of s.values) {
      if (v < low) low = v;
      if (v > high) high = v;
    }
    const pad = (high - low || 1) * 0.08;
    return [includeZero ? Math.min(0, low) : low - pad, high + pad];
  }, [series, includeZero]);

  const x = (t: number) => MARGIN.left + (t / duration) * plotWidth;
  const y = (v: number) => MARGIN.top + (1 - (v - yMin) / (yMax - yMin)) * plotHeight;
  const path = (values: number[]) => {
    const parts: string[] = [];
    for (let i = 0; i < values.length; i += stride) parts.push(`${parts.length ? "L" : "M"}${x(times[i]).toFixed(1)},${y(values[i]).toFixed(1)}`);
    const last = values.length - 1;
    if (last % stride !== 0) parts.push(`L${x(times[last]).toFixed(1)},${y(values[last]).toFixed(1)}`);
    return parts.join("");
  };

  return (
    <figure className="mini-chart">
      <div className="mini-chart-head">
        <span className="mini-chart-title">{title}</span>
        <span className="legend">
          {series
            .filter((s) => !s.quiet)
            .map((s) => (
              <span key={s.label}>
                <svg width={22} height={8} aria-hidden="true">
                  <line x1={0} x2={22} y1={4} y2={4} stroke={s.color} strokeWidth={2} strokeDasharray={s.dash} />
                </svg>
                {s.label}
              </span>
            ))}
        </span>
      </div>
      <svg viewBox={`0 0 ${WIDTH} ${height}`} role="img" aria-label={title}>
        <text className="axis-unit" x={MARGIN.left - 6} y={MARGIN.top + 4} textAnchor="end">
          {unit}
        </text>
        {ticks(yMin, yMax, Y_TICKS).map((v) => (
          <g key={v}>
            <line className="grid" x1={MARGIN.left} x2={WIDTH - MARGIN.right} y1={y(v)} y2={y(v)} />
            <text className="tick" x={MARGIN.left - 6} y={y(v)} dy="0.32em" textAnchor="end">
              {v}
            </text>
          </g>
        ))}
        {ticks(0, duration, X_TICKS).map((t) => (
          <text key={t} className="tick" x={x(t)} y={height - 6} textAnchor="middle">
            {t} s
          </text>
        ))}
        <line className="baseline" x1={MARGIN.left} x2={WIDTH - MARGIN.right} y1={MARGIN.top + plotHeight} y2={MARGIN.top + plotHeight} />
        {marker !== undefined && <line className="playhead" x1={x(marker)} x2={x(marker)} y1={MARGIN.top} y2={MARGIN.top + plotHeight} />}
        {series.map((s) => (
          <path key={s.label} d={path(s.values)} fill="none" stroke={s.color} strokeWidth={s.quiet ? 1 : 1.8} strokeDasharray={s.dash} strokeLinejoin="round" />
        ))}
      </svg>
    </figure>
  );
}
