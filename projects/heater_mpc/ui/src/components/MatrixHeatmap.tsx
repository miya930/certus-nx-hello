import { type PointerEvent, useEffect, useRef, useState } from "react";

interface Props {
  values: number[][];
  label: string;
  /** 値の桁が広いときに、対数で色を付ける。 */
  log?: boolean;
  maxSizePx?: number;
  cellLabel?: (row: number, col: number) => string;
}

// 大きさの表現には 1 つの色相だけを使い、薄いほど小さい値とする。
const RAMP: [number, number, number][] = [
  [205, 226, 251],
  [134, 182, 239],
  [57, 135, 229],
  [37, 106, 191],
  [24, 79, 149],
  [13, 54, 107],
];
const EMPTY: [number, number, number] = [243, 242, 238];
const DEFAULT_MAX_SIZE_PX = 360;
const LOG_FLOOR = 1e-6;

function color(ratio: number): [number, number, number] {
  const position = Math.min(1, Math.max(0, ratio)) * (RAMP.length - 1);
  const lower = Math.min(RAMP.length - 2, Math.floor(position));
  const fraction = position - lower;
  return [0, 1, 2].map((i) => Math.round(RAMP[lower][i] + (RAMP[lower + 1][i] - RAMP[lower][i]) * fraction)) as [
    number,
    number,
    number,
  ];
}

export function MatrixHeatmap({ values, label, log = false, maxSizePx = DEFAULT_MAX_SIZE_PX, cellLabel }: Props) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [hover, setHover] = useState<{ row: number; col: number } | null>(null);
  const rows = values.length;
  const cols = values[0].length;
  const scale = Math.max(1, Math.floor(maxSizePx / Math.max(rows, cols)));
  const widthPx = cols * scale;
  const heightPx = rows * scale;

  let max = -Infinity;
  for (const row of values) {
    for (const v of row) {
      if (v > max) max = v;
    }
  }
  const floor = log ? Math.log10(LOG_FLOOR) : 0;
  const top = log ? Math.log10(Math.max(max, LOG_FLOOR * 10)) : max;
  const ratio = (v: number) => {
    if (v <= 0) return null;
    const value = log ? Math.log10(Math.max(v, LOG_FLOOR)) : v;
    return (value - floor) / (top - floor || 1);
  };

  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    const image = new ImageData(cols, rows);
    values.forEach((row, i) =>
      row.forEach((v, j) => {
        const r = ratio(v);
        const [cr, cg, cb] = r === null ? EMPTY : color(r);
        image.data.set([cr, cg, cb, 255], (i * cols + j) * 4);
      }),
    );
    const source = document.createElement("canvas");
    source.width = cols;
    source.height = rows;
    source.getContext("2d")!.putImageData(image, 0, 0);
    element.width = widthPx;
    element.height = heightPx;
    const context = element.getContext("2d")!;
    context.imageSmoothingEnabled = false;
    context.drawImage(source, 0, 0, widthPx, heightPx);
    // ratio は values と log だけで決まる。
  }, [values, log, rows, cols, widthPx, heightPx]);

  const onPointerMove = (event: PointerEvent<HTMLCanvasElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    const col = Math.min(cols - 1, Math.floor(((event.clientX - rect.left) / rect.width) * cols));
    const row = Math.min(rows - 1, Math.floor(((event.clientY - rect.top) / rect.height) * rows));
    setHover({ row, col });
  };

  const value = hover ? values[hover.row][hover.col] : null;
  return (
    <figure className="matrix">
      <canvas
        ref={canvas}
        style={{ width: widthPx, height: heightPx }}
        role="img"
        aria-label={label}
        onPointerMove={onPointerMove}
        onPointerLeave={() => setHover(null)}
      />
      <figcaption>
        <span>
          {label} · {rows} × {cols}
          {log ? " · log10 color" : ""}
        </span>
        <span className="mono">
          {hover && value !== null
            ? `${cellLabel ? cellLabel(hover.row, hover.col) : `[${hover.row}, ${hover.col}]`} = ${value.toPrecision(4)}`
            : `max ${max.toPrecision(4)}`}
        </span>
      </figcaption>
    </figure>
  );
}
