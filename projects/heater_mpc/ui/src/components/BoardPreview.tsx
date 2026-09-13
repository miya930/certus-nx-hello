import type { Geometry } from "../api";
import { seriesStyle } from "../series";

interface Props {
  geometry: Geometry;
}

const PAD_MM = 12;
const MAX_HEIGHT_PX = 300;

/** Setup で決めた寸法どおりに、基板とヒーターとサーミスタの並びを描く。 */
export function BoardPreview({ geometry: g }: Props) {
  const width = 2 * g.marginMm + (g.cols - 1) * g.pitchMm;
  const height = 2 * g.marginMm + (g.rows - 1) * g.pitchMm;
  const viewW = width + 2 * PAD_MM;
  const viewH = height + 2 * PAD_MM + 8;
  const heaters = Array.from({ length: g.rows * g.cols }, (_, i) => ({
    x: g.marginMm + (i % g.cols) * g.pitchMm,
    y: g.marginMm + Math.floor(i / g.cols) * g.pitchMm,
  }));
  return (
    <figure className="board-preview">
      <svg viewBox={`${-PAD_MM} ${-PAD_MM} ${viewW} ${viewH}`} style={{ maxHeight: MAX_HEIGHT_PX }} role="img" aria-label={`${width} × ${height} mm の基板に ${g.rows} 行 ${g.cols} 列のヒーター`}>
        <rect x={0} y={0} width={width} height={height} className="preview-board" />
        {heaters.map((h, i) => (
          <g key={i}>
            <rect x={h.x - g.heaterXMm / 2} y={h.y - g.heaterYMm / 2} width={g.heaterXMm} height={g.heaterYMm} rx={0.4} className="preview-heater" style={{ stroke: seriesStyle(i).color }} />
            <circle cx={h.x} cy={h.y + g.sensorOffsetMm} r={0.9} className="preview-sensor" />
            <text x={h.x} y={h.y - g.heaterYMm / 2 - 1.5} textAnchor="middle" className="preview-label">
              {seriesStyle(i).label}
            </text>
          </g>
        ))}
        <line x1={0} x2={width} y1={height + 5} y2={height + 5} className="preview-dim" />
        <text x={width / 2} y={height + 4} textAnchor="middle" className="preview-label">
          {width} mm
        </text>
        <line x1={-5} x2={-5} y1={0} y2={height} className="preview-dim" />
        <text x={-6.5} y={height / 2} textAnchor="middle" className="preview-label" transform={`rotate(-90 ${-6.5} ${height / 2})`}>
          {height} mm
        </text>
        {g.cols > 1 && (
          <>
            <line x1={heaters[0].x} x2={heaters[1].x} y1={-5} y2={-5} className="preview-dim" />
            <text x={(heaters[0].x + heaters[1].x) / 2} y={-6.5} textAnchor="middle" className="preview-label">
              間隔 {g.pitchMm} mm
            </text>
          </>
        )}
      </svg>
      <figcaption>
        四角がヒーター (2512 サイズなら 6 × 3 mm)、点がサーミスタ。端までの距離 {g.marginMm} mm、厚さ {g.thicknessMm} mm、銅箔 {Math.round(g.copper * 100)} %。
      </figcaption>
    </figure>
  );
}
