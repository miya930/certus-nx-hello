import { useMemo, useState } from "react";
import { makeChain } from "../playground/chain";
import { runEstimation } from "../playground/simulate";
import { seriesStyle } from "../series";
import { useDebounced } from "../useDebounced";
import { CellStrip } from "./CellStrip";
import { MiniChart } from "./MiniChart";
import { Slider } from "./Slider";
import { Tex } from "./Tex";

const UNMEASURED_CELL = 5;
const WATCHED_SENSOR = 1;
const TRUTH_COLOR = "var(--text-primary)";
const MODEL_COLOR = "var(--text-muted)";

/** 開ループで電力を入れた 1 次元の模型に対して、カルマンフィルタの推定を実際の温度と比べる。 */
export function KalmanPlayground() {
  const [noiseStd, setNoiseStd] = useState(0.1);
  const [modelScale, setModelScale] = useState(0.8);
  const [stateNoise, setStateNoise] = useState(1e-4);
  const [disturbanceNoise, setDisturbanceNoise] = useState(1e-2);
  const [measurementNoise, setMeasurementNoise] = useState(1e-2);
  const [useDisturbance, setUseDisturbance] = useState(true);
  const [sampleTime, setSampleTime] = useState(5);
  const [stepIndex, setStepIndex] = useState(40);
  const settings = useDebounced({ noiseStd, modelScale, stateNoise, disturbanceNoise, measurementNoise, useDisturbance, sampleTime }, 150);
  const run = useMemo(() => runEstimation(settings), [settings]);
  const chain = useMemo(() => makeChain(), []);

  const k = Math.min(stepIndex, run.times.length - 1);
  const step = run.steps[k];
  const filter = run.filter;
  const nx = filter.states;
  const sensorCell = chain.sensors[WATCHED_SENSOR];
  const max = Math.max(...run.truth.map((row) => Math.max(...row)), 1);
  const cellSeries = [
    { label: "実際の温度", values: run.truth.map((r) => r[UNMEASURED_CELL]), color: TRUTH_COLOR },
    { label: "モデルだけの予測", values: run.modelOnly.map((r) => r[UNMEASURED_CELL]), color: MODEL_COLOR, dash: "5 4" },
    { label: "推定 x̂", values: run.estimate.map((r) => r[UNMEASURED_CELL]), color: "var(--accent)" },
  ];
  const sensorSeries = [
    { label: "実際の温度", values: run.truth.map((r) => r[sensorCell]), color: TRUTH_COLOR },
    { label: "測定 y (雑音あり)", values: run.measurement.map((r) => r[WATCHED_SENSOR]), color: seriesStyle(WATCHED_SENSOR).color, quiet: false },
    { label: "推定 x̂ + d̂", values: run.estimate.map((r, i) => r[sensorCell] + (run.disturbance[i][WATCHED_SENSOR] ?? 0)), color: "var(--accent)" },
  ];
  const disturbanceSeries = Array.from({ length: filter.sensors }, (_, i) => ({
    label: `d̂ ${seriesStyle(i).label}`,
    values: run.disturbance.map((r) => r[i] ?? 0),
    color: seriesStyle(i).color,
  }));
  const format = (v: number) => (Math.abs(v) < 0.0005 && v !== 0 ? v.toExponential(1) : v.toFixed(3));
  const formatGain = (v: number) => (Math.abs(v) < 0.01 ? (Math.abs(v) < 1e-9 ? "0" : v.toExponential(2)) : v.toFixed(3));

  return (
    <div className="playground">
      <div className="playground-controls">
        <Slider label="測定雑音の標準偏差" value={noiseStd} min={0} max={0.5} step={0.05} unit="K" onChange={setNoiseStd} />
        <Slider label="モデルのずれ (熱コンダクタンスの倍率)" value={modelScale} min={0.6} max={1.4} step={0.05} onChange={setModelScale} />
        <Slider label="Q_x 状態の雑音の分散" value={stateNoise} min={1e-6} max={1e-1} log onChange={setStateNoise} />
        <Slider label="Q_d 外乱の雑音の分散" value={disturbanceNoise} min={1e-5} max={1} log onChange={setDisturbanceNoise} />
        <Slider label="R 測定雑音の分散" value={measurementNoise} min={1e-4} max={1} log onChange={setMeasurementNoise} />
        <Slider label="Ts 制御周期" value={sampleTime} min={1} max={20} step={1} unit="s" digits={0} onChange={setSampleTime} />
        <label className="check">
          <input type="checkbox" checked={useDisturbance} onChange={(e) => setUseDisturbance(e.target.checked)} />
          出力外乱 d の状態を持つ (外すとモデルのずれが定常偏差として残る)
        </label>
        <p className="muted small">
          ヒーターには決まった電力 (0.15 W、0.3 W、0.15 W、300 s で中央を 0 W) を入れ、フィルタはサーミスタ 3 点だけを見る。
          リッカチ方程式の反復は {filter.iterations} 回で収束し、L は {filter.gain.length} × {filter.sensors} である。
        </p>
      </div>
      <div className="playground-charts">
        <MiniChart title={`測っていない格子 x[${UNMEASURED_CELL}]`} unit="K" times={run.times} series={cellSeries} marker={run.times[k]} />
        <MiniChart title={`サーミスタ ${seriesStyle(WATCHED_SENSOR).label} の格子 x[${sensorCell}]`} unit="K" times={run.times} series={sensorSeries} height={150} marker={run.times[k]} />
        {useDisturbance && <MiniChart title="推定した出力外乱 d̂" unit="K" times={run.times} series={disturbanceSeries} height={120} marker={run.times[k]} />}
      </div>

      <div className="playground-step">
        <Slider label="計算例を見る時刻 k" value={k} min={0} max={run.times.length - 1} step={1} unit={`(t = ${run.times[k]} s)`} digits={0} onChange={setStepIndex} />
        <CellStrip label="実際の温度" values={run.truth[k]} max={max} sensors={chain.sensors} heaters={chain.heaters} />
        <CellStrip label="推定 x̂_k" values={run.estimate[k]} max={max} sensors={chain.sensors} heaters={chain.heaters} />
        <p className="muted small">上の帯は基板の 15 格子で、色の濃さが温度上昇。色の棒がヒーター、下の点がサーミスタの位置。</p>

        <h4>
          時刻 k = {k} (t = {run.times[k]} s) の 1 周期の計算
        </h4>
        <p>
          前の周期の推定 <Tex math={String.raw`\hat\xi_{k-1}`} /> を <Tex math={String.raw`A_\xi`} /> で進め、前の周期の電力 <Tex math={`u_{k-1} = [${run.input[Math.max(0, k - 1)].map((v) => v.toFixed(2)).join(",\\ ")}]`} /> W の効果を足したものが予測である。
          予測から期待される測定 <Tex math={String.raw`\hat{y} = C_\xi \hat\xi_{k|k-1}`} /> と、実際の測定 <Tex math="y_k" /> の差にゲインをかけて足す。
        </p>
        <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th />
                {chain.sensors.map((cell, i) => (
                  <th key={i}>
                    {seriesStyle(i).label} (x[{cell}])
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  予測 <Tex math={String.raw`\hat{x}_{k|k-1}`} /> の格子の値
                </td>
                {chain.sensors.map((cell, i) => (
                  <td key={i}>{format(step.predicted[cell])}</td>
                ))}
              </tr>
              {useDisturbance && (
                <tr>
                  <td>
                    予測 <Tex math={String.raw`\hat{d}_{k|k-1}`} />
                  </td>
                  {chain.sensors.map((_, i) => (
                    <td key={i}>{format(step.predicted[nx + i])}</td>
                  ))}
                </tr>
              )}
              <tr>
                <td>
                  期待される測定 <Tex math={String.raw`\hat{y} = \hat{x} + \hat{d}`} />
                </td>
                {step.expected.map((v, i) => (
                  <td key={i}>{format(v)}</td>
                ))}
              </tr>
              <tr>
                <td>
                  測定 <Tex math="y_k" />
                </td>
                {run.measurement[k].map((v, i) => (
                  <td key={i}>{format(v)}</td>
                ))}
              </tr>
              <tr className="highlight-row">
                <td>
                  差 <Tex math={String.raw`y_k - \hat{y}`} /> (イノベーション)
                </td>
                {step.innovation.map((v, i) => (
                  <td key={i}>{format(v)}</td>
                ))}
              </tr>
              <tr>
                <td>
                  補正 <Tex math={String.raw`L (y_k - \hat{y})`} /> の格子の値
                </td>
                {chain.sensors.map((cell, i) => (
                  <td key={i}>{format(step.correction[cell])}</td>
                ))}
              </tr>
              {useDisturbance && (
                <tr>
                  <td>
                    補正 <Tex math={String.raw`L (y_k - \hat{y})`} /> の <Tex math="d" /> の値
                  </td>
                  {chain.sensors.map((_, i) => (
                    <td key={i}>{format(step.correction[nx + i])}</td>
                  ))}
                </tr>
              )}
              <tr className="highlight-row">
                <td>
                  更新 <Tex math={String.raw`\hat{x}_k`} /> の格子の値
                </td>
                {chain.sensors.map((cell, i) => (
                  <td key={i}>{format(step.updated[cell])}</td>
                ))}
              </tr>
              {useDisturbance && (
                <tr className="highlight-row">
                  <td>
                    更新 <Tex math={String.raw`\hat{d}_k`} />
                  </td>
                  {chain.sensors.map((_, i) => (
                    <td key={i}>{format(step.updated[nx + i])}</td>
                  ))}
                </tr>
              )}
              <tr>
                <td>実際の温度 (参考)</td>
                {chain.sensors.map((cell, i) => (
                  <td key={i}>{format(run.truth[k][cell])}</td>
                ))}
              </tr>
            </tbody>
          </table>
        </div>
        <p className="muted small">
          補正は測った格子だけでなく全ての格子に配られる。測っていない格子 x[{UNMEASURED_CELL}] への補正は{" "}
          {formatGain(step.correction[UNMEASURED_CELL])} K で、その行の L は [{filter.gain[UNMEASURED_CELL].map(formatGain).join(", ")}] である。
          状態の雑音 <Tex math="Q_x" /> が外乱の雑音 <Tex math="Q_d" /> より小さい既定の設定では、差のほとんどが <Tex math="d" /> に配られ、格子の値はほぼモデルで決まる。
        </p>

        <h4>定常カルマンゲイン L (行が状態、列がセンサー)</h4>
        <div className="table-scroll">
          <table className="data-table gain-table">
            <thead>
              <tr>
                <th>状態</th>
                {chain.sensors.map((_, i) => (
                  <th key={i}>{seriesStyle(i).label}</th>
                ))}
                <th>
                  予測誤差の標準偏差 <Tex math={String.raw`\sqrt{P_{ii}}`} />
                </th>
              </tr>
            </thead>
            <tbody>
              {filter.gain.map((row, i) => (
                <tr key={i} className={chain.sensors.includes(i) ? "highlight-row" : ""}>
                  <td>{i < nx ? `x[${i}]${chain.sensors.includes(i) ? " (センサー)" : ""}` : `d ${seriesStyle(i - nx).label}`}</td>
                  {row.map((v, j) => (
                    <td key={j}>{formatGain(v)}</td>
                  ))}
                  <td>{formatGain(filter.predictedStd[i])}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
