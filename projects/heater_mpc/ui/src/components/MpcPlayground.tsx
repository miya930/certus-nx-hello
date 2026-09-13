import { useMemo, useState } from "react";
import { runMpc, runPid } from "../playground/simulate";
import { seriesStyle } from "../series";
import { useDebounced } from "../useDebounced";
import { MiniChart } from "./MiniChart";
import { Slider } from "./Slider";

const POWER_MAX = 0.5;
const SV_COLOR = "var(--text-secondary)";
const SV_DASH = "5 4";
const PID_DASH = "2 3";
const PID_GAINS = { kp: 0.08, ti: 60, td: 1 };

/** 1 次元の模型で MPC の設定を動かし、PID との違いを見る。 */
export function MpcPlayground() {
  const [horizon, setHorizon] = useState(40);
  const [controlHorizon, setControlHorizon] = useState(8);
  const [moveWeight, setMoveWeight] = useState(100);
  const [sampleTime, setSampleTime] = useState(5);
  const [ramp, setRamp] = useState(0);
  const [modelScale, setModelScale] = useState(1);
  const [comparePid, setComparePid] = useState(true);
  const settings = useDebounced({ horizon, controlHorizon: Math.min(controlHorizon, horizon), moveWeight, sampleTime, ramp, modelScale }, 150);

  const mpc = useMemo(
    () =>
      runMpc(
        { horizon: settings.horizon, controlHorizon: settings.controlHorizon, moveWeight: settings.moveWeight, powerMax: POWER_MAX },
        settings.sampleTime,
        settings.ramp,
        settings.modelScale,
      ),
    [settings],
  );
  const pid = useMemo(() => runPid(PID_GAINS, POWER_MAX, settings.ramp), [settings.ramp]);

  const heaters = mpc.pv[0].length;
  const pvSeries = [
    ...Array.from({ length: heaters }, (_, h) => ({ label: `SV ${seriesStyle(h).label}`, values: mpc.sv.map((r) => r[h]), color: SV_COLOR, dash: SV_DASH, quiet: h > 0 })),
    ...(comparePid
      ? Array.from({ length: heaters }, (_, h) => ({ label: `PID ${seriesStyle(h).label}`, values: pid.pv.map((r) => r[h]), color: seriesStyle(h).color, dash: PID_DASH, quiet: h > 0 }))
      : []),
    ...Array.from({ length: heaters }, (_, h) => ({ label: `MPC ${seriesStyle(h).label}`, values: mpc.pv.map((r) => r[h]), color: seriesStyle(h).color })),
  ];
  const mvSeries = Array.from({ length: heaters }, (_, h) => ({ label: seriesStyle(h).label, values: mpc.mv.map((r) => r[h]), color: seriesStyle(h).color }));

  return (
    <div className="playground">
      <div className="playground-controls">
        <Slider label="Ts 制御周期" value={sampleTime} min={1} max={20} step={1} unit="s" digits={0} onChange={setSampleTime} />
        <Slider label="Np 予測ホライズン" value={horizon} min={5} max={100} step={5} unit={`steps = ${horizon * sampleTime} s`} digits={0} onChange={setHorizon} />
        <Slider label="Nc 制御ホライズン" value={Math.min(controlHorizon, horizon)} min={1} max={Math.min(20, horizon)} step={1} unit="steps" digits={0} onChange={setControlHorizon} />
        <Slider label="λ 操作量の変化の重み" value={moveWeight} min={1} max={3000} log onChange={setMoveWeight} />
        <Slider label="予測モデルのずれ (熱コンダクタンスの倍率)" value={modelScale} min={0.6} max={1.4} step={0.05} onChange={setModelScale} />
        <Slider label="昇温レート (0 = ステップ)" value={ramp} min={0} max={20} step={1} unit="K/min" digits={0} onChange={setRamp} />
        <label className="check">
          <input type="checkbox" checked={comparePid} onChange={(e) => setComparePid(e.target.checked)} />
          PID (Kp 0.08, Ti 60 s, Td 1 s) を点線で重ねる
        </label>
        <p className="muted small">
          平均 |SV − PV|: MPC {mpc.meanAbsoluteError.toFixed(2)} K{comparePid ? `、PID ${pid.meanAbsoluteError.toFixed(2)} K` : ""}。
          λ を小さくすると速く追従するが電力が激しく動く。Ts を長くすると誤差が増える。モデルのずれは外乱の推定が吸収し、定常では目標に一致する。
        </p>
      </div>
      <div className="playground-charts">
        <MiniChart title="PV と SV" unit="°C" times={mpc.times} series={pvSeries} />
        <MiniChart title="MPC の MV (電力)" unit="W" times={mpc.times} series={mvSeries} height={140} includeZero />
      </div>
    </div>
  );
}
