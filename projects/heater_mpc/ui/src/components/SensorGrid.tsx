import { seriesStyle } from "../series";
import { LineKey } from "./LineKey";

interface Props {
  cols: number;
  pv: number[];
  sv: number[];
  mv: number[];
  hovered: number | null;
  selected: number;
  onHover: (index: number | null) => void;
  onSelect: (index: number) => void;
}

/** 基板と同じ並びで各ヒーターの値を示す。凡例と、現在時刻の値の表を兼ねる。 */
export function SensorGrid(props: Props) {
  return (
    <div className="sensor-grid" style={{ gridTemplateColumns: `repeat(${props.cols}, 1fr)` }}>
      {props.pv.map((value, i) => {
        const style = seriesStyle(i);
        const state = i === props.selected ? "selected" : i === props.hovered ? "hovered" : "";
        return (
          <button
            key={i}
            type="button"
            className={`sensor-cell ${state}`}
            onPointerEnter={() => props.onHover(i)}
            onPointerLeave={() => props.onHover(null)}
            onFocus={() => props.onHover(i)}
            onBlur={() => props.onHover(null)}
            onClick={() => props.onSelect(i)}
            aria-pressed={i === props.selected}
          >
            <span className="sensor-cell-head">
              <LineKey color={style.color} />
              {style.label}
            </span>
            <span className="sensor-cell-value">{value.toFixed(1)} °C</span>
            <span className="sensor-cell-sub">
              SV {props.sv[i].toFixed(1)} · MV {props.mv[i].toFixed(2)} W
            </span>
          </button>
        );
      })}
    </div>
  );
}
