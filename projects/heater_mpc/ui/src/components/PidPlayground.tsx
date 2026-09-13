import { useMemo, useState } from "react";
import { runPid } from "../playground/simulate";
import { seriesStyle } from "../series";
import { useDebounced } from "../useDebounced";
import { MiniChart } from "./MiniChart";
import { Slider } from "./Slider";

const POWER_MAX = 0.5;
const SV_COLOR = "var(--text-secondary)";
const SV_DASH = "5 4";

/** 1 次元の模型で PID のゲインを動かし、応答の変化を見る。 */
export function PidPlayground() {
  const [kp, setKp] = useState(0.08);
  const [ti, setTi] = useState(60);
  const [td, setTd] = useState(1);
  const [ramp, setRamp] = useState(0);
  const settings = useDebounced({ kp, ti, td, ramp }, 120);
  const trace = useMemo(() => runPid({ kp: settings.kp, ti: settings.ti, td: settings.td }, POWER_MAX, settings.ramp), [settings]);

  const heaters = trace.pv[0].length;
  const pvSeries = [
    ...Array.from({ length: heaters }, (_, h) => ({ label: `SV ${seriesStyle(h).label}`, values: trace.sv.map((r) => r[h]), color: SV_COLOR, dash: SV_DASH, quiet: h > 0 })),
    ...Array.from({ length: heaters }, (_, h) => ({ label: `PV ${seriesStyle(h).label}`, values: trace.pv.map((r) => r[h]), color: seriesStyle(h).color })),
  ];
  const mvSeries = Array.from({ length: heaters }, (_, h) => ({ label: seriesStyle(h).label, values: trace.mv.map((r) => r[h]), color: seriesStyle(h).color }));

  return (
    <div className="playground">
      <div className="playground-controls">
        <Slider label="Kp 比例ゲイン" value={kp} min={0.005} max={1} log unit="W/K" onChange={setKp} />
        <Slider label="Ti 積分時間" value={ti} min={5} max={600} log unit="s" onChange={setTi} />
        <Slider label="Td 微分時間" value={td} min={0} max={20} step={0.5} unit="s" digits={1} onChange={setTd} />
        <Slider label="昇温レート (0 = ステップ)" value={ramp} min={0} max={20} step={1} unit="K/min" digits={0} onChange={setRamp} />
        <p className="muted small">
          平均 |SV − PV| = {trace.meanAbsoluteError.toFixed(2)} K。Kp を上げると速くなるが、200 s で中央だけ目標を上げたときに両側が引きずられて揺れる。
          Ti を短くすると隣からの熱を早く打ち消すが、行き過ぎが増える。
        </p>
      </div>
      <div className="playground-charts">
        <MiniChart title="PV と SV" unit="°C" times={trace.times} series={pvSeries} />
        <MiniChart title="MV (電力)" unit="W" times={trace.times} series={mvSeries} height={140} includeZero />
      </div>
    </div>
  );
}
