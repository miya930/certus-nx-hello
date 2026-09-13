import { useMemo, useState } from "react";
import { seriesStyle } from "../series";
import { LineKey } from "./LineKey";
import { TimeSeriesChart } from "./TimeSeriesChart";

interface Props {
  label: string;
  unit: string;
  digits: number;
  cols: number;
  timeStep: number;
  values: number[][];
  phaseStarts: number[];
  time: number;
  highlight: number | null;
  onSeek: (time: number) => void;
  includeZero?: boolean;
}

/** 全ヒーターの時系列を重ね、カーソル位置の値を基板と同じ並びで示す。 */
export function OverlayChart(props: Props) {
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);
  // 再生中は毎フレーム描き直すため、線の形を作り直さないよう系列を固定する。
  const series = useMemo(
    () =>
      props.values.map((values, i) => {
        const style = seriesStyle(i);
        return { key: style.label, values, color: style.color };
      }),
    [props.values],
  );

  const readout = (index: number) => (
    <div className="tooltip-grid" style={{ gridTemplateColumns: `repeat(${props.cols}, auto)` }}>
      {series.map((s, i) => (
        <div key={s.key} className={`tooltip-cell ${i === props.highlight ? "highlight" : ""}`}>
          <strong>
            {s.values[index].toFixed(props.digits)} {props.unit}
          </strong>
          <span>
            <LineKey color={s.color} /> {s.key}
          </span>
        </div>
      ))}
    </div>
  );

  return (
    <TimeSeriesChart
      label={props.label}
      unit={props.unit}
      timeStep={props.timeStep}
      series={series}
      phaseStarts={props.phaseStarts}
      cursorTime={props.time}
      highlight={props.highlight === null ? null : series[props.highlight].key}
      hoverIndex={hoverIndex}
      onHoverIndex={setHoverIndex}
      onSeek={props.onSeek}
      tooltip={readout}
      includeZero={props.includeZero}
    />
  );
}
