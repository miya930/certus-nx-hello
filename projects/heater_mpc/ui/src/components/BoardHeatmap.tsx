import { type PointerEvent, useEffect, useRef, useState } from "react";
import type { SimulationResult } from "../api";
import { heatColor } from "../heatColor";
import { seriesStyle } from "../series";
import { useElementWidth } from "../useElementWidth";

interface Props {
  result: SimulationResult;
  frame: number[];
  domain: [number, number];
  pv: number[];
  sv: number[];
  mv: number[];
  hovered: number | null;
  selected: number;
  onHover: (index: number | null) => void;
  onSelect: (index: number) => void;
}

const MAX_SIZE_PX = 460;
// ヒーターのホバー判定は、部品の外形より広い正方形にして当てやすくする。
const HIT_SIZE_MM = 10;
const SENSOR_RADIUS_MM = 0.9;

interface Pointer {
  xPx: number;
  yPx: number;
  xMm: number;
  yMm: number;
}

export function BoardHeatmap(props: Props) {
  const { result, frame, domain } = props;
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const canvas = useRef<HTMLCanvasElement>(null);
  const [pointer, setPointer] = useState<Pointer | null>(null);
  const { widthMm, heightMm, heaterSizeMm } = result.board;
  const { rows, cols, cellMm, scale: valueScale } = result.frame;
  // 基板の縦横比を保ったまま、枠に収まる大きさにする。
  const pxPerMm = Math.min(Math.min(width, MAX_SIZE_PX) / widthMm, MAX_SIZE_PX / heightMm);
  const widthPx = widthMm * pxPerMm;
  const heightPx = heightMm * pxPerMm;
  const half = result.plant.dxMm / 2;

  useEffect(() => {
    const element = canvas.current;
    if (!element || widthPx === 0) {
      return;
    }
    const image = new ImageData(cols, rows);
    frame.forEach((value, i) => {
      const [r, g, b] = heatColor((value / valueScale - domain[0]) / (domain[1] - domain[0]));
      image.data.set([r, g, b, 255], i * 4);
    });
    const source = document.createElement("canvas");
    source.width = cols;
    source.height = rows;
    source.getContext("2d")!.putImageData(image, 0, 0);

    const ratio = window.devicePixelRatio || 1;
    element.width = widthPx * ratio;
    element.height = heightPx * ratio;
    const context = element.getContext("2d")!;
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = "high";
    context.drawImage(source, 0, 0, cols * cellMm * pxPerMm * ratio, rows * cellMm * pxPerMm * ratio);
  }, [frame, domain, widthPx, heightPx, pxPerMm, rows, cols, cellMm, valueScale]);

  const heaterAt = (xMm: number, yMm: number) =>
    result.board.heaters.findIndex(
      (h) => Math.abs(xMm - h.x - half) <= HIT_SIZE_MM / 2 && Math.abs(yMm - h.y - half) <= HIT_SIZE_MM / 2,
    );

  const onPointerMove = (event: PointerEvent<SVGSVGElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    const xPx = event.clientX - rect.left;
    const yPx = event.clientY - rect.top;
    const next = { xPx, yPx, xMm: xPx / pxPerMm, yMm: yPx / pxPerMm };
    setPointer(next);
    const index = heaterAt(next.xMm, next.yMm);
    props.onHover(index >= 0 ? index : null);
  };

  const onPointerLeave = () => {
    setPointer(null);
    props.onHover(null);
  };

  const pointedTemperature = pointer
    ? frame[
        Math.min(rows - 1, Math.floor(pointer.yMm / cellMm)) * cols + Math.min(cols - 1, Math.floor(pointer.xMm / cellMm))
      ] / valueScale
    : null;

  return (
    <div className="heatmap" ref={ref}>
      <div className="heatmap-stage" style={{ width: widthPx, height: heightPx }}>
        <canvas ref={canvas} style={{ width: widthPx, height: heightPx }} />
        <svg
          viewBox={`0 0 ${widthMm} ${heightMm}`}
          width={widthPx}
          height={heightPx}
          role="img"
          aria-label="Board temperature map"
          onPointerMove={onPointerMove}
          onPointerLeave={onPointerLeave}
          onClick={() => props.hovered !== null && props.onSelect(props.hovered)}
        >
          <rect className="board-outline" x={0} y={0} width={widthMm} height={heightMm} />
          {result.board.heaters.map((h, i) => {
            const sensor = result.board.sensors[i];
            const state = i === props.selected ? "selected" : i === props.hovered ? "hovered" : "";
            return (
              <g key={i} className={`heater ${state}`}>
                <rect
                  x={h.x + half - heaterSizeMm.x / 2}
                  y={h.y + half - heaterSizeMm.y / 2}
                  width={heaterSizeMm.x}
                  height={heaterSizeMm.y}
                  rx={0.4}
                />
                <circle className="sensor" cx={sensor.x + half} cy={sensor.y + half} r={SENSOR_RADIUS_MM} />
                <text x={h.x + half} y={h.y + half - heaterSizeMm.y / 2 - 1.2} textAnchor="middle">
                  {seriesStyle(i).label}
                </text>
              </g>
            );
          })}
        </svg>
        {pointer && pointedTemperature !== null && (
          <div
            className="tooltip"
            style={{
              left: pointer.xPx,
              top: pointer.yPx,
              transform: `translate(${pointer.xPx > widthPx * 0.6 ? "calc(-100% - 12px)" : "12px"}, 12px)`,
            }}
          >
            {props.hovered !== null ? (
              <>
                <div className="tooltip-title">{seriesStyle(props.hovered).label}</div>
                <div className="tooltip-row">
                  <strong>{props.pv[props.hovered].toFixed(2)} °C</strong>
                  <span>PV</span>
                </div>
                <div className="tooltip-row">
                  <strong>{props.sv[props.hovered].toFixed(1)} °C</strong>
                  <span>SV</span>
                </div>
                <div className="tooltip-row">
                  <strong>{props.mv[props.hovered].toFixed(3)} W</strong>
                  <span>MV</span>
                </div>
              </>
            ) : (
              <>
                <div className="tooltip-row">
                  <strong>{pointedTemperature.toFixed(1)} °C</strong>
                </div>
                <div className="tooltip-row">
                  <span>
                    x {pointer.xMm.toFixed(0)} mm, y {pointer.yMm.toFixed(0)} mm
                  </span>
                </div>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
