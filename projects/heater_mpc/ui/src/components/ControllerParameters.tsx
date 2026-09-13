import type { ControllerName, SimulationResult } from "../api";
import { Tex } from "./Tex";

interface Props {
  name: ControllerName;
  result: SimulationResult;
}

interface Row {
  symbol: string;
  label: string;
  value: string;
}

function pidRows(result: SimulationResult): Row[] {
  const info = result.controllers.PID.info;
  const tuning: Row[] =
    info.mode === "imc"
      ? [
          { symbol: "\\lambda / \\tau", label: "IMC の閉ループ時定数の比", value: (info.lambdaRatio ?? 0).toFixed(2) },
          { symbol: "\\lambda", label: "IMC の閉ループ時定数", value: `${(info.closedLoopTime ?? 0).toFixed(1)} s` },
          { symbol: "", label: "調整", value: info.optimized ? `IMC、λ を ${info.sweep?.length ?? 0} 点掃引` : "IMC、λ 固定" },
        ]
      : [{ symbol: "", label: "調整", value: "ゲインを直接指定" }];
  return [
    { symbol: "K_p", label: "比例ゲイン", value: `${info.kp.toFixed(4)} W/K` },
    { symbol: "T_i", label: "積分時間", value: `${info.ti.toFixed(1)} s` },
    { symbol: "T_d", label: "微分時間", value: `${info.td.toFixed(2)} s` },
    { symbol: "N", label: "微分フィルタの係数", value: info.derivativeFilter.toFixed(0) },
    { symbol: "T_s", label: "制御周期", value: `${info.sampleTime.toFixed(0)} s` },
    { symbol: "u", label: "操作量の範囲", value: `0 … ${result.powerMax.toFixed(2)} W` },
    { symbol: "K", label: "FOPDT のゲイン", value: `${info.fopdt.gain.toFixed(2)} K/W` },
    { symbol: "\\tau", label: "FOPDT の時定数", value: `${info.fopdt.tau.toFixed(1)} s` },
    { symbol: "\\theta", label: "FOPDT のむだ時間", value: `${info.fopdt.delay.toFixed(1)} s` },
    ...tuning,
  ];
}

function mpcRows(result: SimulationResult): Row[] {
  const info = result.controllers.MPC.info;
  const heaters = result.rows * result.cols;
  return [
    { symbol: "T_s", label: "制御周期", value: `${info.sampleTime.toFixed(0)} s` },
    {
      symbol: "N_p",
      label: "予測ホライズン",
      value: `${info.horizon} steps (${(info.horizon * info.sampleTime).toFixed(0)} s)`,
    },
    {
      symbol: "N_c",
      label: "制御ホライズン",
      value: `${info.controlHorizon} steps (${(info.controlHorizon * info.sampleTime).toFixed(0)} s)`,
    },
    { symbol: "\\lambda", label: "操作量の変化の重み", value: info.moveWeight.toFixed(0) },
    { symbol: "r", label: "参照軌道", value: `${info.horizon} steps 先まで先読み` },
    { symbol: "n_x", label: "予測モデルの状態数", value: `${info.states} (${info.modelDxMm.toFixed(0)} mm 格子)` },
    { symbol: "n_u, n_y", label: "操作量と出力の数", value: `${heaters}, ${heaters}` },
    { symbol: "u", label: "操作量の制約", value: `0 … ${result.powerMax.toFixed(2)} W` },
    { symbol: "Q_x", label: "状態の雑音の分散", value: info.stateNoise.toExponential(0) },
    { symbol: "Q_d", label: "出力外乱の雑音の分散", value: info.disturbanceNoise.toExponential(0) },
    { symbol: "R", label: "測定雑音の分散", value: info.measurementNoise.toExponential(0) },
  ];
}

export function ControllerParameters({ name, result }: Props) {
  const rows = name === "PID" ? pidRows(result) : mpcRows(result);
  return (
    <table className="parameter-table">
      <tbody>
        {rows.map((row) => (
          <tr key={row.label}>
            <td className="parameter-symbol">{row.symbol && <Tex math={row.symbol} />}</td>
            <td>{row.label}</td>
            <td className="parameter-value">{row.value}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
