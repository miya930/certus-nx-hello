import { useMemo, useState } from "react";
import type { ControllerName, SimulationResult } from "../api";
import { seriesStyle } from "../series";
import { LineKey } from "./LineKey";
import { TimeSeriesChart } from "./TimeSeriesChart";

interface Props {
  result: SimulationResult;
  name: ControllerName;
  selected: number;
  onSelect: (index: number) => void;
  time: number;
  onSeek: (time: number) => void;
}

const CHART_HEIGHT = 200;
const SV_DASH = "6 4";
const SV_COLOR = "var(--text-secondary)";

/** 1 つのヒーターの PV、SV、MV を、時間軸をそろえた 2 つのグラフで示す。 */
export function LoopPanel({ result, name, selected, onSelect, time, onSeek }: Props) {
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);
  const controller = result.controllers[name];
  const style = seriesStyle(selected);
  const phaseStarts = result.phases.map((p) => p.start);

  // 再生中は毎フレーム描き直すため、線の形を作り直さないよう系列を固定する。
  const { pv, sv, mv, temperatureSeries, powerSeries } = useMemo(() => {
    const pv = controller.pv.map((row) => row[selected]);
    const sv = result.sv.map((row) => row[selected]);
    const mv = controller.mv.map((row) => row[selected]);
    return {
      pv,
      sv,
      mv,
      temperatureSeries: [
        { key: "sv", values: sv, color: SV_COLOR, dash: SV_DASH },
        { key: "pv", values: pv, color: style.color },
      ],
      powerSeries: [{ key: "mv", values: mv, color: style.color }],
    };
  }, [controller, result, selected, style.color]);

  const readout = (index: number) => (
    <>
      <div className="tooltip-row">
        <LineKey color={style.color} />
        <strong>{pv[index].toFixed(2)} °C</strong>
        <span>PV</span>
      </div>
      <div className="tooltip-row">
        <LineKey color={SV_COLOR} dash={SV_DASH} />
        <strong>{sv[index].toFixed(2)} °C</strong>
        <span>SV</span>
      </div>
      <div className="tooltip-row">
        <strong>{(sv[index] - pv[index]).toFixed(2)} K</strong>
        <span>e = SV − PV</span>
      </div>
      <div className="tooltip-row">
        <strong>{mv[index].toFixed(3)} W</strong>
        <span>MV</span>
      </div>
    </>
  );

  return (
    <section className="card">
      <header className="card-header">
        <h2>
          ループ {style.label} <span className="muted">PV / SV / MV</span>
        </h2>
        <label className="inline-field">
          ヒーター
          <select value={selected} onChange={(event) => onSelect(Number(event.target.value))}>
            {controller.pv[0].map((_, i) => (
              <option key={i} value={i}>
                {seriesStyle(i).label}
              </option>
            ))}
          </select>
        </label>
      </header>
      <div className="legend">
        <span>
          <LineKey color={style.color} /> PV
        </span>
        <span>
          <LineKey color={SV_COLOR} dash={SV_DASH} /> SV
        </span>
      </div>
      <TimeSeriesChart
        label={`${style.label} process and set values`}
        unit="°C"
        timeStep={result.timeStep}
        series={temperatureSeries}
        phaseStarts={phaseStarts}
        cursorTime={time}
        highlight={null}
        hoverIndex={hoverIndex}
        onHoverIndex={setHoverIndex}
        onSeek={onSeek}
        tooltip={readout}
        height={CHART_HEIGHT}
      />
      <div className="chart-caption">MV (ヒーターの電力)</div>
      <TimeSeriesChart
        label={`${style.label} manipulated value`}
        unit="W"
        timeStep={result.timeStep}
        series={powerSeries}
        phaseStarts={phaseStarts}
        cursorTime={time}
        highlight={null}
        hoverIndex={hoverIndex}
        onHoverIndex={setHoverIndex}
        onSeek={onSeek}
        tooltip={readout}
        height={CHART_HEIGHT}
        includeZero
      />
    </section>
  );
}
