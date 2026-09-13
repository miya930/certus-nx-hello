import { type KeyboardEvent, type PointerEvent, type ReactNode, useMemo } from "react";
import { useElementWidth } from "../useElementWidth";

export interface ChartSeries {
  key: string;
  values: number[];
  color: string;
  dash?: string;
}

interface Props {
  label: string;
  unit: string;
  timeStep: number;
  series: ChartSeries[];
  phaseStarts: number[];
  cursorTime: number;
  highlight: string | null;
  hoverIndex: number | null;
  onHoverIndex: (index: number | null) => void;
  onSeek: (time: number) => void;
  tooltip: (index: number) => ReactNode;
  height?: number;
  includeZero?: boolean;
}

const MARGIN = { top: 12, right: 16, bottom: 28, left: 48 };
const DEFAULT_HEIGHT = 240;
const X_TICKS = 6;
const Y_TICKS = 5;
const Y_PADDING = 0.08;
const KEY_STEP_SECONDS = 10;
const DIMMED_OPACITY = 0.18;
// 近い範囲の値を 1 本の縦線にまとめるときも、スパイクを消さないよう最小値と最大値を残す。
const POINTS_PER_PIXEL = 1;

function niceStep(span: number, count: number) {
  const raw = span / count;
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const normalized = raw / magnitude;
  const step = normalized < 1.5 ? 1 : normalized < 3 ? 2 : normalized < 7 ? 5 : 10;
  return step * magnitude;
}

function ticks(min: number, max: number, count: number) {
  const step = niceStep(max - min, count);
  const values: number[] = [];
  for (let v = Math.ceil(min / step) * step; v <= max + step * 1e-6; v += step) {
    values.push(Number(v.toFixed(10)));
  }
  return values;
}

function linePath(values: number[], x: (i: number) => number, y: (v: number) => number, buckets: number) {
  const size = Math.max(1, Math.ceil(values.length / buckets));
  const points: number[] = [];
  for (let start = 0; start < values.length; start += size) {
    const end = Math.min(values.length, start + size);
    let low = start;
    let high = start;
    for (let i = start; i < end; i++) {
      if (values[i] < values[low]) low = i;
      if (values[i] > values[high]) high = i;
    }
    points.push(...(low < high ? [low, high] : low === high ? [low] : [high, low]));
  }
  return points.map((i, n) => `${n === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(values[i]).toFixed(1)}`).join("");
}

export function TimeSeriesChart(props: Props) {
  const { series, timeStep, height = DEFAULT_HEIGHT, hoverIndex, onHoverIndex } = props;
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const samples = series[0]?.values.length ?? 0;
  const duration = (samples - 1) * timeStep;
  const plotWidth = Math.max(0, width - MARGIN.left - MARGIN.right);
  const plotHeight = height - MARGIN.top - MARGIN.bottom;

  const [yMin, yMax] = useMemo(() => {
    let low = props.includeZero ? 0 : Infinity;
    let high = -Infinity;
    for (const s of series) {
      for (const v of s.values) {
        low = Math.min(low, v);
        high = Math.max(high, v);
      }
    }
    const pad = (high - low || 1) * Y_PADDING;
    return [props.includeZero ? low : low - pad, high + pad];
  }, [series, props.includeZero]);

  const x = (i: number) => MARGIN.left + (duration ? ((i * timeStep) / duration) * plotWidth : 0);
  const y = (v: number) => MARGIN.top + (1 - (v - yMin) / (yMax - yMin)) * plotHeight;

  const paths = useMemo(
    () => series.map((s) => linePath(s.values, x, y, plotWidth * POINTS_PER_PIXEL)),
    // x と y は描画範囲だけで決まるため、その依存で足りる。
    [series, plotWidth, plotHeight, yMin, yMax, duration],
  );

  const indexAt = (clientX: number, element: Element) => {
    const offset = clientX - element.getBoundingClientRect().left - MARGIN.left;
    const time = Math.min(duration, Math.max(0, (offset / plotWidth) * duration));
    return Math.round(time / timeStep);
  };

  const onPointerMove = (event: PointerEvent<SVGSVGElement>) =>
    onHoverIndex(indexAt(event.clientX, event.currentTarget));

  const onKeyDown = (event: KeyboardEvent<SVGSVGElement>) => {
    const current = hoverIndex ?? Math.round(props.cursorTime / timeStep);
    const step = Math.round(KEY_STEP_SECONDS / timeStep);
    if (event.key === "ArrowRight") onHoverIndex(Math.min(samples - 1, current + step));
    else if (event.key === "ArrowLeft") onHoverIndex(Math.max(0, current - step));
    else if (event.key === "Enter") props.onSeek(current * timeStep);
    else if (event.key === "Escape") onHoverIndex(null);
    else return;
    event.preventDefault();
  };

  const ordered = series
    .map((s, i) => ({ s, path: paths[i] }))
    .sort((a, b) => Number(a.s.key === props.highlight) - Number(b.s.key === props.highlight));
  const hoverX = hoverIndex === null ? null : x(hoverIndex);

  return (
    <div className="chart" ref={ref}>
      {width > 0 && (
        <svg
          width={width}
          height={height}
          role="img"
          aria-label={props.label}
          tabIndex={0}
          onPointerMove={onPointerMove}
          onPointerLeave={() => onHoverIndex(null)}
          onClick={(event) => props.onSeek(indexAt(event.clientX, event.currentTarget) * timeStep)}
          onKeyDown={onKeyDown}
          onFocus={() => onHoverIndex(Math.round(props.cursorTime / timeStep))}
          onBlur={() => onHoverIndex(null)}
        >
          <text className="axis-unit" x={MARGIN.left - 8} y={MARGIN.top - 2} textAnchor="end">
            {props.unit}
          </text>
          {ticks(yMin, yMax, Y_TICKS).map((v) => (
            <g key={`y${v}`}>
              <line className="grid" x1={MARGIN.left} x2={MARGIN.left + plotWidth} y1={y(v)} y2={y(v)} />
              <text className="tick" x={MARGIN.left - 8} y={y(v)} dy="0.32em" textAnchor="end">
                {v}
              </text>
            </g>
          ))}
          {duration > 0 &&
            ticks(0, duration, X_TICKS).map((t) => (
              <text key={`x${t}`} className="tick" x={x(t / timeStep)} y={height - 8} textAnchor="middle">
                {t} s
              </text>
            ))}
          <line
            className="baseline"
            x1={MARGIN.left}
            x2={MARGIN.left + plotWidth}
            y1={MARGIN.top + plotHeight}
            y2={MARGIN.top + plotHeight}
          />
          {props.phaseStarts
            .filter((t) => t > 0)
            .map((t) => (
              <line
                key={`p${t}`}
                className="phase"
                x1={x(t / timeStep)}
                x2={x(t / timeStep)}
                y1={MARGIN.top}
                y2={MARGIN.top + plotHeight}
              />
            ))}
          {ordered.map(({ s, path }) => (
            <path
              key={s.key}
              d={path}
              fill="none"
              stroke={s.color}
              strokeWidth={2}
              strokeDasharray={s.dash}
              strokeLinejoin="round"
              opacity={props.highlight && s.key !== props.highlight ? DIMMED_OPACITY : 1}
            />
          ))}
          <line
            className="playhead"
            x1={x(props.cursorTime / timeStep)}
            x2={x(props.cursorTime / timeStep)}
            y1={MARGIN.top}
            y2={MARGIN.top + plotHeight}
          />
          {hoverIndex !== null && hoverX !== null && (
            <g>
              <line className="crosshair" x1={hoverX} x2={hoverX} y1={MARGIN.top} y2={MARGIN.top + plotHeight} />
              {ordered.map(({ s }) => (
                <circle
                  key={s.key}
                  cx={hoverX}
                  cy={y(s.values[hoverIndex])}
                  r={4}
                  fill={s.color}
                  className="marker"
                  opacity={props.highlight && s.key !== props.highlight ? DIMMED_OPACITY : 1}
                />
              ))}
            </g>
          )}
        </svg>
      )}
      {hoverIndex !== null && hoverX !== null && (
        <div
          className="tooltip"
          style={{
            left: hoverX,
            top: MARGIN.top,
            transform: hoverX > width * 0.6 ? "translateX(calc(-100% - 12px))" : "translateX(12px)",
          }}
        >
          <div className="tooltip-time">t = {(hoverIndex * timeStep).toFixed(0)} s</div>
          {props.tooltip(hoverIndex)}
        </div>
      )}
    </div>
  );
}
