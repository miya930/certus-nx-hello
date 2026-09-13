import { useState } from "react";
import type { Geometry, JobProgress, OptimizeTarget, PidMode, SimulationRequest } from "../api";
import { type PresetName, presetPhases, resizeSetpoints } from "../presets";
import { BoardPreview } from "./BoardPreview";
import { NumberField } from "./NumberField";
import { PhaseEditor } from "./PhaseEditor";
import { ReferencePreview } from "./ReferencePreview";

interface Props {
  request: SimulationRequest;
  running: boolean;
  optimizing: boolean;
  progress: JobProgress | null;
  open: boolean;
  onToggle: () => void;
  onChange: (request: SimulationRequest) => void;
  onRun: () => void;
  onOptimize: (target: OptimizeTarget) => void;
}

// サーバーが受け付ける範囲に合わせる。
const GRID_SIZES = [1, 2, 3, 4];
const DURATIONS = [600, 900, 1800, 2700, 3600];
const MAX_BOARD_MM = 150;
const MODEL_DX_MM = 5;
const OPTIMIZE_TARGETS: { value: OptimizeTarget; label: string }[] = [
  { value: "both", label: "PID と MPC" },
  { value: "pid", label: "PID だけ" },
  { value: "mpc", label: "MPC だけ" },
];

export function boardSize(g: Geometry) {
  return {
    width: 2 * g.marginMm + (g.cols - 1) * g.pitchMm,
    height: 2 * g.marginMm + (g.rows - 1) * g.pitchMm,
  };
}

export function SetupPanel({ request, running, optimizing, progress, open, onToggle, onChange, onRun, onOptimize }: Props) {
  const [preset, setPreset] = useState<PresetName>("steps");
  const [target, setTarget] = useState<OptimizeTarget>("both");
  const { geometry, scenario, pid, mpc } = request;
  const size = boardSize(geometry);
  const heaters = geometry.rows * geometry.cols;
  const tooLarge = Math.max(size.width, size.height) > MAX_BOARD_MM;
  const heaterTooLarge = Math.max(geometry.heaterXMm, geometry.heaterYMm) >= geometry.pitchMm;
  const busy = running || optimizing;
  const invalid = tooLarge || heaterTooLarge;
  const modelStates = Math.round(size.width / MODEL_DX_MM) * Math.round(size.height / MODEL_DX_MM);

  const setGeometry = (change: Partial<Geometry>) => {
    const next = { ...geometry, ...change };
    const count = next.rows * next.cols;
    // ヒーターの数が変わったら、目標温度の列数をそろえる。プリセットならその形で作り直す。
    const phases =
      count === heaters
        ? scenario.phases
        : preset === "custom"
          ? resizeSetpoints(scenario.phases, count)
          : presetPhases(preset, next.rows, next.cols, scenario.duration);
    onChange({ ...request, geometry: next, scenario: { ...scenario, phases } });
  };

  const setScenario = (change: Partial<typeof scenario>) => onChange({ ...request, scenario: { ...scenario, ...change } });
  const setPid = (change: Partial<typeof pid>) => onChange({ ...request, pid: { ...pid, ...change } });
  const setMpc = (change: Partial<typeof mpc>) => onChange({ ...request, mpc: { ...mpc, ...change } });

  const applyPreset = (name: PresetName, duration = scenario.duration) => {
    setPreset(name);
    if (name !== "custom") {
      setScenario({ duration, phases: presetPhases(name, geometry.rows, geometry.cols, duration) });
    } else {
      // 段階の開始時刻が新しい長さを超えないようにする。
      setScenario({ duration, phases: scenario.phases.map((p) => ({ ...p, start: Math.min(p.start, duration) })) });
    }
  };

  const summary =
    `${geometry.rows}×${geometry.cols}、間隔 ${geometry.pitchMm} mm、${size.width} × ${size.height} mm、厚さ ${geometry.thicknessMm} mm、銅箔 ${Math.round(geometry.copper * 100)} %、` +
    `最大 ${request.powerMax} W、PID ${pid.mode === "imc" ? `IMC λ/τ = ${pid.lambdaRatio}` : "ゲイン直接指定"}、` +
    `MPC Ts ${mpc.sampleTime} s / Np ${mpc.horizon} / Nc ${mpc.controlHorizon} / λ ${mpc.moveWeight}、` +
    `${scenario.duration} s、${scenario.rampRate > 0 ? `${scenario.rampRate} K/min` : "ステップ"}`;

  return (
    <section className="card setup">
      <header className="card-header">
        <button type="button" className="disclosure" onClick={onToggle} aria-expanded={open}>
          <span className="disclosure-arrow">{open ? "▾" : "▸"}</span>
          <h2>シミュレーションの設定</h2>
        </button>
        {!open && <span className="muted setup-summary">{summary}</span>}
        {!open && (
          <button type="button" className="ghost-button tall" onClick={onToggle}>
            設定を開く
          </button>
        )}
      </header>

      {busy && (
        <div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round((progress?.fraction ?? 0) * 100)}>
          <div className="progress-track">
            <div className="progress-bar" style={{ width: `${(progress?.fraction ?? 0) * 100}%` }} />
          </div>
          <span className="progress-label mono">
            {progress?.stage ?? "starting"} · {Math.round((progress?.fraction ?? 0) * 100)} %
          </span>
        </div>
      )}

      <div hidden={!open}>
        <p className="muted setup-intro">
          3 つの段で条件を決める。右の図は入力に合わせてその場で描き直されるので、基板の並びと目標温度の形を確かめてから実行する。
        </p>
        <div className="setup-layout">
          <div className="setup-form">
            <fieldset className="setup-group">
              <legend>1. 基板</legend>
              <p className="group-note">FR4 の 2 層基板に、チップ抵抗のヒーターとサーミスタを格子に並べる。間隔を狭めるか銅箔を増やすと、隣のヒーターの熱が強く届く。</p>
              <div className="field-row">
                <label className="field">
                  <span className="field-label">行数</span>
                  <span className="field-control">
                    <select value={geometry.rows} onChange={(e) => setGeometry({ rows: Number(e.target.value) })}>
                      {GRID_SIZES.map((n) => (
                        <option key={n} value={n}>
                          {n}
                        </option>
                      ))}
                    </select>
                  </span>
                </label>
                <label className="field">
                  <span className="field-label">列数</span>
                  <span className="field-control">
                    <select value={geometry.cols} onChange={(e) => setGeometry({ cols: Number(e.target.value) })}>
                      {GRID_SIZES.map((n) => (
                        <option key={n} value={n}>
                          {n}
                        </option>
                      ))}
                    </select>
                  </span>
                </label>
                <NumberField label="ヒーター間隔" value={geometry.pitchMm} min={5} max={60} step={1} unit="mm" onChange={(v) => setGeometry({ pitchMm: v })} />
                <NumberField label="端までの距離" value={geometry.marginMm} min={3} max={40} step={1} unit="mm" onChange={(v) => setGeometry({ marginMm: v })} />
              </div>
              <div className="field-row">
                <NumberField label="ヒーター外形 X" value={geometry.heaterXMm} min={1} max={12} step={0.5} unit="mm" onChange={(v) => setGeometry({ heaterXMm: v })} />
                <NumberField label="ヒーター外形 Y" value={geometry.heaterYMm} min={1} max={12} step={0.5} unit="mm" onChange={(v) => setGeometry({ heaterYMm: v })} />
                <NumberField label="サーミスタの位置 (中心からの距離)" value={geometry.sensorOffsetMm} min={0} max={15} step={0.5} unit="mm" onChange={(v) => setGeometry({ sensorOffsetMm: v })} />
              </div>
              <div className="field-row">
                <NumberField label="基板の厚さ" value={geometry.thicknessMm} min={0.4} max={3.2} step={0.1} unit="mm" onChange={(v) => setGeometry({ thicknessMm: v })} />
                <NumberField label="銅箔の割合 (各層)" value={Math.round(geometry.copper * 100)} min={0} max={100} step={10} unit="%" onChange={(v) => setGeometry({ copper: v / 100 })} />
              </div>
              {tooLarge && <div className="field-error">基板は 1 辺 {MAX_BOARD_MM} mm までにする。</div>}
              {heaterTooLarge && <div className="field-error">ヒーターの外形は間隔より小さくする。</div>}
            </fieldset>

            <fieldset className="setup-group">
              <legend>2. ヒーターと制御器</legend>
              <p className="group-note">最大電力はヒーター 1 個の上限で、PID と MPC の両方に同じ制約として入る。</p>
              <div className="field-row">
                <NumberField label="ヒーター 1 個の最大電力" value={request.powerMax} min={0.05} max={2} step={0.05} unit="W" onChange={(v) => onChange({ ...request, powerMax: v })} />
              </div>
              <div className="group-caption">PID (ヒーターごとに独立)</div>
              <p className="group-note">IMC は、中央ヒーターのステップ応答を一次遅れとむだ時間で近似し、閉ループの時定数 λ から Kp、Ti、Td を決める。λ/τ が小さいほど速い。</p>
              <div className="field-row">
                <label className="field">
                  <span className="field-label">調整方法</span>
                  <span className="field-control">
                    <select value={pid.mode} onChange={(e) => setPid({ mode: e.target.value as PidMode })}>
                      <option value="imc">ステップ応答から IMC で決める</option>
                      <option value="explicit">Kp、Ti、Td を直接指定</option>
                    </select>
                  </span>
                </label>
                {pid.mode === "imc" ? (
                  <>
                    <NumberField label="λ / τ" value={pid.lambdaRatio} min={0.1} max={5} step={0.05} onChange={(v) => setPid({ lambdaRatio: v })} />
                    <label className="field checkbox-field">
                      <span className="field-label">λ の掃引</span>
                      <span className="field-control">
                        <input type="checkbox" checked={pid.optimize} onChange={(e) => setPid({ optimize: e.target.checked })} />
                        0.25 … 4 を試して誤差最小を選ぶ
                      </span>
                    </label>
                  </>
                ) : (
                  <>
                    <NumberField label="Kp" value={pid.kp} min={0.0001} max={10} step={0.001} unit="W/K" onChange={(v) => setPid({ kp: v })} />
                    <NumberField label="Ti" value={pid.ti} min={1} max={3600} step={1} unit="s" onChange={(v) => setPid({ ti: v })} />
                    <NumberField label="Td" value={pid.td} min={0} max={600} step={0.1} unit="s" onChange={(v) => setPid({ td: v })} />
                  </>
                )}
              </div>
              <div className="group-caption">MPC (全ヒーターをまとめて最適化)</div>
              <p className="group-note">
                Ts ごとに Np ステップ先まで予測し、Nc ステップ分の電力を決める。λ は電力の急な変化を抑える重みで、小さいほど速く追従し電力が激しく動く。
              </p>
              <div className="field-row">
                <NumberField label="制御周期 Ts" value={mpc.sampleTime} min={1} max={30} step={1} unit="s" onChange={(v) => setMpc({ sampleTime: v })} />
                <NumberField label="予測ホライズン Np" value={mpc.horizon} min={5} max={120} step={5} unit={`steps = ${mpc.horizon * mpc.sampleTime} s`} onChange={(v) => setMpc({ horizon: v, controlHorizon: Math.min(mpc.controlHorizon, v) })} />
                <NumberField label="制御ホライズン Nc" value={mpc.controlHorizon} min={1} max={mpc.horizon} step={1} unit={`steps = ${mpc.controlHorizon * mpc.sampleTime} s`} onChange={(v) => setMpc({ controlHorizon: v })} />
                <NumberField label="操作量の変化の重み λ" value={mpc.moveWeight} min={0} max={10000} step={10} onChange={(v) => setMpc({ moveWeight: v })} />
              </div>
            </fieldset>

            <fieldset className="setup-group">
              <legend>3. 参照軌道 (目標温度の時間変化)</legend>
              <p className="group-note">
                目標温度は段階ごとに与え、開始時刻で切り替える。昇温レートを正にすると、周囲温度から始めてその速さで目標へ移る傾斜になる。
              </p>
              <div className="field-row">
                <label className="field">
                  <span className="field-label">シミュレーションの時間</span>
                  <span className="field-control">
                    <select value={scenario.duration} onChange={(e) => applyPreset(preset, Number(e.target.value))}>
                      {DURATIONS.map((d) => (
                        <option key={d} value={d}>
                          {d} s
                        </option>
                      ))}
                    </select>
                  </span>
                </label>
                <NumberField label="昇温レート (0 = ステップ)" value={scenario.rampRate} min={0} max={60} step={0.5} unit="K/min" onChange={(v) => setScenario({ rampRate: v })} />
              </div>
              <PhaseEditor
                heaters={heaters}
                duration={scenario.duration}
                phases={scenario.phases}
                preset={preset}
                onPreset={(name) => applyPreset(name)}
                onChange={(phases) => {
                  setPreset("custom");
                  setScenario({ phases });
                }}
              />
            </fieldset>
          </div>

          <aside className="setup-preview">
            <h3>基板の並び</h3>
            <BoardPreview geometry={geometry} />
            <h3>参照軌道</h3>
            <ReferencePreview scenario={scenario} heaters={heaters} />
            <h3>計算の規模</h3>
            <ul className="preview-facts">
              <li>
                基板のモデル: 1 mm 格子で {size.width * size.height} 状態、1 s 刻みで {scenario.duration} ステップ
              </li>
              <li>
                MPC の予測モデル: {MODEL_DX_MM} mm 格子で約 {modelStates} 状態、QP は {mpc.controlHorizon * heaters} 変数 × {mpc.horizon * heaters} 予測値
              </li>
              <li>QP を解く回数: {Math.floor(scenario.duration / mpc.sampleTime)} 回 (Ts ごと)</li>
            </ul>
          </aside>
        </div>

        <footer className="setup-actions-bar">
          <div className="setup-optimize">
            <select value={target} onChange={(e) => setTarget(e.target.value as OptimizeTarget)} aria-label="最適化の対象" disabled={busy}>
              {OPTIMIZE_TARGETS.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
            <button type="button" className="ghost-button tall" onClick={() => onOptimize(target)} disabled={busy || invalid}>
              {optimizing ? "最適化中…" : "パラメータを最適化"}
            </button>
            <span className="muted small">PID は Kp、Ti、Td を、MPC は λ と Nc を、参照軌道に対する誤差が最小になるように探して 2 の欄に書き戻す。約 1 分。</span>
          </div>
          <button type="button" className="run-button" onClick={onRun} disabled={busy || invalid}>
            {running ? "計算中…" : "シミュレーションを実行"}
          </button>
        </footer>
      </div>
    </section>
  );
}
