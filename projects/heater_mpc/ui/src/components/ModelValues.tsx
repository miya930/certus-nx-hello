import type { SimulationResult, StateSpaceModel } from "../api";
import { seriesStyle } from "../series";
import { MatrixHeatmap } from "./MatrixHeatmap";
import { Tex } from "./Tex";

interface Props {
  result: SimulationResult;
}

const ROW_ENTRIES = 9;
const COLUMN_ENTRIES = 9;
const SLOWEST_MODES = 5;
const FASTEST_MODES = 3;
const NEGLIGIBLE = 1e-6;

function cellName(model: StateSpaceModel, index: number) {
  return `x[${index}] (row ${Math.floor(index / model.cols)}, col ${index % model.cols})`;
}

function offset(model: StateSpaceModel, from: number, to: number) {
  const dr = Math.floor(to / model.cols) - Math.floor(from / model.cols);
  const dc = (to % model.cols) - (from % model.cols);
  return dr === 0 && dc === 0 ? "self" : `${dr >= 0 ? "+" : ""}${dr} row, ${dc >= 0 ? "+" : ""}${dc} col`;
}

function download(model: StateSpaceModel) {
  const payload = {
    description: "Discrete-time state-space model x[k+1] = A x[k] + B u[k], y[k] = x[k][sensorCells]",
    units: { x: "K above ambient", u: "W", sampleTime: "s" },
    ...model,
  };
  const blob = new Blob([JSON.stringify(payload)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `heater_mpc_model_${model.rows}x${model.cols}_Ts${model.sampleTime}s.json`;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function ModelValues({ result }: Props) {
  const model = result.controllers.MPC.model;
  const heaters = result.rows * result.cols;
  const center = Math.floor(result.rows / 2) * result.cols + Math.floor(result.cols / 2);
  const centerCell = model.heaterCells[center];
  const row = model.a[centerCell];
  const rowEntries = row
    .map((value, j) => ({ j, value }))
    .filter((e) => Math.abs(e.value) > NEGLIGIBLE)
    .sort((p, q) => q.value - p.value);
  const column = model.b.map((r) => r[center]);
  const columnEntries = column
    .map((value, i) => ({ i, value }))
    .filter((e) => e.value > NEGLIGIBLE)
    .sort((p, q) => q.value - p.value);
  const columnSum = column.reduce((s, v) => s + v, 0);
  const storage = model.cellCapacity / model.sampleTime;
  const interiorDiagonal = 4 * model.conductance + model.cellLoss;
  const tau = model.timeConstants;

  return (
    <>
      <h3>この基板でのモデルの値</h3>
      <p>
        MPC の予測モデルは <Tex math={String.raw`\Delta x = ${model.dxMm}`} /> mm 格子で、{model.rows} 行 {model.cols} 列の{" "}
        <Tex math={`n_x = ${model.states}`} /> 状態、<Tex math={`n_u = n_y = ${heaters}`} /> である。
        状態 <Tex math="x[i]" /> は格子 <Tex math="i = \text{row} \cdot n_\text{cols} + \text{col}" /> の周囲温度からの上昇 [K] で、
        入力 <Tex math="u" /> はヒーターの電力 [W] である。
      </p>
      <p>連続時間の係数は、格子 1 つあたり次の値になる。</p>
      <table className="data-table">
        <tbody>
          <tr>
            <td>
              <Tex math={String.raw`C_\text{cell} = C'' \Delta x^2`} /> 熱容量
            </td>
            <td>{model.cellCapacity.toPrecision(4)} J/K</td>
          </tr>
          <tr>
            <td>
              <Tex math="G" /> 隣の格子への熱コンダクタンス
            </td>
            <td>{model.conductance.toPrecision(4)} W/K</td>
          </tr>
          <tr>
            <td>
              <Tex math={String.raw`2h \Delta x^2`} /> 両面からの放熱のコンダクタンス
            </td>
            <td>{model.cellLoss.toPrecision(4)} W/K</td>
          </tr>
          <tr>
            <td>
              <Tex math={String.raw`\mathbf{K}_{ii}`} /> 内部の格子の対角成分 <Tex math={String.raw`= 4G + 2h\Delta x^2`} />
            </td>
            <td>{interiorDiagonal.toPrecision(4)} W/K</td>
          </tr>
          <tr>
            <td>
              <Tex math={String.raw`C_\text{cell} / T_s`} /> 離散化で対角に足す項 (<Tex math={`T_s = ${model.sampleTime}`} /> s)
            </td>
            <td>{storage.toPrecision(4)} W/K</td>
          </tr>
          <tr>
            <td>
              <Tex math={String.raw`\tau_\text{cell} = C_\text{cell} / \mathbf{K}_{ii}`} /> 格子 1 つの時定数の目安
            </td>
            <td>{(model.cellCapacity / interiorDiagonal).toFixed(1)} s</td>
          </tr>
        </tbody>
      </table>
      <p>
        <Tex math={String.raw`A = (C_\text{cell} I / T_s + \mathbf{K})^{-1} \, C_\text{cell} / T_s`} /> は逆行列のため密行列になるが、
        値は距離とともに急に小さくなる。<Tex math={String.raw`B = (C_\text{cell} I / T_s + \mathbf{K})^{-1} \mathbf{Q}`} /> も同じである。
        各行の和は 1 より少し小さく、その差が 1 ステップで両面から逃げる割合である。
      </p>
      <div className="matrix-row">
        <MatrixHeatmap values={model.a} label="A" log cellLabel={(i, j) => `A[${i}, ${j}]`} />
        <MatrixHeatmap values={model.b} label="B" log maxSizePx={360} cellLabel={(i, j) => `B[${i}, ${seriesStyle(j).label}]`} />
      </div>
      <div className="model-columns">
        <div>
          <h4>
            <Tex math="A" /> の {seriesStyle(center).label} 直下の格子の行 ({cellName(model, centerCell)})
          </h4>
          <table className="data-table">
            <thead>
              <tr>
                <th>j</th>
                <th>位置</th>
                <th>
                  <Tex math="A_{ij}" />
                </th>
              </tr>
            </thead>
            <tbody>
              {rowEntries.slice(0, ROW_ENTRIES).map((e) => (
                <tr key={e.j}>
                  <td>{e.j}</td>
                  <td>{offset(model, centerCell, e.j)}</td>
                  <td>{e.value.toFixed(6)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted small">
            {NEGLIGIBLE.toExponential(0)} より大きい成分は {rowEntries.length} 個、行の和は {row.reduce((s, v) => s + v, 0).toFixed(6)}。
          </p>
        </div>
        <div>
          <h4>
            <Tex math="B" /> の {seriesStyle(center).label} の列
          </h4>
          <table className="data-table">
            <thead>
              <tr>
                <th>i</th>
                <th>位置</th>
                <th>
                  <Tex math="B_{i}" /> [K/W]
                </th>
              </tr>
            </thead>
            <tbody>
              {columnEntries.slice(0, COLUMN_ENTRIES).map((e) => (
                <tr key={e.i}>
                  <td>{e.i}</td>
                  <td>{offset(model, centerCell, e.i)}</td>
                  <td>{e.value.toFixed(6)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted small">
            列の和は {columnSum.toFixed(4)} K/W で、1 W を 1 ステップ加えたときに基板に残る温度上昇の合計である。
          </p>
        </div>
        <div>
          <h4>
            <Tex math="C" /> とモード
          </h4>
          <p>
            <Tex math="C" /> は各サーミスタの格子を取り出す行列で、<Tex math="y_i = x[s_i]" /> である。
          </p>
          <table className="data-table">
            <thead>
              <tr>
                <th>出力</th>
                <th>
                  <Tex math="s_i" />
                </th>
                <th>ヒーターの格子</th>
              </tr>
            </thead>
            <tbody>
              {model.sensorCells.map((s, i) => (
                <tr key={i}>
                  <td>{seriesStyle(i).label}</td>
                  <td>{s}</td>
                  <td>{model.heaterCells[i]}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p>
            <Tex math="A" /> の固有値 <Tex math={String.raw`\mu`} /> を時定数 <Tex math={String.raw`\tau = -T_s / \ln \mu`} /> に直すと、遅いモードは{" "}
            {tau.slice(0, SLOWEST_MODES).map((t) => t.toFixed(0)).join(", ")} s、速いモードは{" "}
            {tau.slice(-FASTEST_MODES).map((t) => t.toFixed(1)).join(", ")} s である。
            最も遅いモードが基板全体の温まり方で、PID の調整に使った一次遅れの時定数 {result.controllers.PID.info.fopdt.tau.toFixed(0)} s はその中間にある。
          </p>
        </div>
      </div>
      <button type="button" className="ghost-button" onClick={() => download(model)}>
        Download A, B, sensor cells as JSON
      </button>
    </>
  );
}
