import { seriesStyle } from "../series";

interface Props {
  label: string;
  values: number[];
  max: number;
  sensors: number[];
  heaters: number[];
}

const CELL_W = 40;
const CELL_H = 30;
const LABEL_W = 70;

/** 1 次元の基板の各格子の温度上昇を、色の濃さと数値で並べて示す。 */
export function CellStrip({ label, values, max, sensors, heaters }: Props) {
  const width = LABEL_W + values.length * CELL_W;
  return (
    <svg className="cell-strip" viewBox={`0 0 ${width} ${CELL_H + 14}`} role="img" aria-label={`${label} の各格子の温度上昇`}>
      <text x={0} y={CELL_H / 2 + 4} className="strip-label">
        {label}
      </text>
      {values.map((v, i) => {
        const heater = heaters.indexOf(i);
        return (
          <g key={i}>
            <rect x={LABEL_W + i * CELL_W} y={0} width={CELL_W} height={CELL_H} className="strip-cell" style={{ fillOpacity: max > 0 ? Math.min(1, Math.max(0, v / max)) : 0 }} />
            <text x={LABEL_W + i * CELL_W + CELL_W / 2} y={CELL_H / 2 + 4} textAnchor="middle" className="strip-value">
              {v.toFixed(1)}
            </text>
            {heater >= 0 && <rect x={LABEL_W + i * CELL_W + 4} y={2} width={CELL_W - 8} height={4} rx={1} style={{ fill: seriesStyle(heater).color }} />}
            {sensors.includes(i) && <circle cx={LABEL_W + i * CELL_W + CELL_W / 2} cy={CELL_H + 8} r={3} className="strip-sensor" />}
          </g>
        );
      })}
    </svg>
  );
}
