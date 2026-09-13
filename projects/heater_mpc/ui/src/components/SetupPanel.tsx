import { useState } from "react";
import type { Geometry, JobProgress, OptimizeTarget, PidMode, SimulationRequest } from "../api";
import { type PresetName, presetPhases, resizeSetpoints } from "../presets";
import { NumberField } from "./NumberField";
import { PhaseEditor } from "./PhaseEditor";

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
const OPTIMIZE_TARGETS: { value: OptimizeTarget; label: string }[] = [
  { value: "both", label: "PID and MPC" },
  { value: "pid", label: "PID only" },
  { value: "mpc", label: "MPC only" },
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
  const tooLarge = Math.max(size.width, size.height) > MAX_BOARD_MM;
  const heaterTooLarge = Math.max(geometry.heaterXMm, geometry.heaterYMm) >= geometry.pitchMm;
  const busy = running || optimizing;
  const invalid = tooLarge || heaterTooLarge;

  const setGeometry = (change: Partial<Geometry>) => {
    const next = { ...geometry, ...change };
    const heaters = next.rows * next.cols;
    // ヒーターの数が変わったら、目標温度の列数をそろえる。プリセットならその形で作り直す。
    const phases =
      heaters === geometry.rows * geometry.cols
        ? scenario.phases
        : preset === "custom"
          ? resizeSetpoints(scenario.phases, heaters)
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
    `${geometry.rows}×${geometry.cols}, pitch ${geometry.pitchMm} mm, ${size.width} × ${size.height} mm, ` +
    `${geometry.thicknessMm} mm, copper ${Math.round(geometry.copper * 100)} %, ${request.powerMax} W max, ` +
    `PID ${pid.mode === "imc" ? `IMC λ/τ ${pid.lambdaRatio}` : "explicit gains"}, ` +
    `MPC Ts ${mpc.sampleTime} s Np ${mpc.horizon} Nc ${mpc.controlHorizon} λ ${mpc.moveWeight}, ` +
    `${scenario.duration} s, ${scenario.rampRate > 0 ? `${scenario.rampRate} K/min` : "step"}`;

  return (
    <section className="card setup">
      <header className="card-header">
        <button type="button" className="disclosure" onClick={onToggle} aria-expanded={open}>
          <span className="disclosure-arrow">{open ? "▾" : "▸"}</span>
          <h2>Setup</h2>
        </button>
        {!open && <span className="muted setup-summary">{summary}</span>}
        <div className="setup-actions">
          <select value={target} onChange={(e) => setTarget(e.target.value as OptimizeTarget)} aria-label="Optimization target" disabled={busy}>
            {OPTIMIZE_TARGETS.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
          <button type="button" className="ghost-button tall" onClick={() => onOptimize(target)} disabled={busy || invalid}>
            {optimizing ? "Optimizing…" : "Optimize"}
          </button>
          <button type="button" className="run-button" onClick={onRun} disabled={busy || invalid}>
            {running ? "Simulating…" : "Run simulation"}
          </button>
        </div>
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
        <div className="setup-columns">
          <fieldset className="setup-group">
            <legend>Board geometry</legend>
            <div className="field-row">
              <label className="field">
                <span className="field-label">Rows</span>
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
                <span className="field-label">Columns</span>
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
              <NumberField label="Heater pitch" value={geometry.pitchMm} min={5} max={60} step={1} unit="mm" onChange={(v) => setGeometry({ pitchMm: v })} />
              <NumberField label="Edge margin" value={geometry.marginMm} min={3} max={40} step={1} unit="mm" onChange={(v) => setGeometry({ marginMm: v })} />
            </div>
            <div className="field-row">
              <NumberField label="Heater size X" value={geometry.heaterXMm} min={1} max={12} step={0.5} unit="mm" onChange={(v) => setGeometry({ heaterXMm: v })} />
              <NumberField label="Heater size Y" value={geometry.heaterYMm} min={1} max={12} step={0.5} unit="mm" onChange={(v) => setGeometry({ heaterYMm: v })} />
              <NumberField label="Sensor offset" value={geometry.sensorOffsetMm} min={0} max={15} step={0.5} unit="mm" onChange={(v) => setGeometry({ sensorOffsetMm: v })} />
            </div>
            <div className="field-row">
              <NumberField label="Thickness (FR4)" value={geometry.thicknessMm} min={0.4} max={3.2} step={0.1} unit="mm" onChange={(v) => setGeometry({ thicknessMm: v })} />
              <NumberField label="Copper per layer" value={Math.round(geometry.copper * 100)} min={0} max={100} step={10} unit="%" onChange={(v) => setGeometry({ copper: v / 100 })} />
              <div className="field">
                <span className="field-label">Board</span>
                <span className={`field-control mono ${tooLarge ? "invalid" : ""}`}>
                  {size.width} × {size.height} mm
                </span>
              </div>
            </div>
            {tooLarge && <div className="field-error">Board must be at most {MAX_BOARD_MM} mm on each side.</div>}
            {heaterTooLarge && <div className="field-error">Heater must be smaller than the pitch.</div>}
          </fieldset>

          <fieldset className="setup-group">
            <legend>Heaters and controllers</legend>
            <div className="field-row">
              <NumberField label="Max heater power" value={request.powerMax} min={0.05} max={2} step={0.05} unit="W" onChange={(v) => onChange({ ...request, powerMax: v })} />
            </div>
            <div className="group-caption">PID</div>
            <div className="field-row">
              <label className="field">
                <span className="field-label">Tuning</span>
                <span className="field-control">
                  <select value={pid.mode} onChange={(e) => setPid({ mode: e.target.value as PidMode })}>
                    <option value="imc">IMC from step response</option>
                    <option value="explicit">Explicit gains</option>
                  </select>
                </span>
              </label>
              {pid.mode === "imc" ? (
                <>
                  <NumberField label="λ / τ" value={pid.lambdaRatio} min={0.1} max={5} step={0.05} onChange={(v) => setPid({ lambdaRatio: v })} />
                  <label className="field checkbox-field">
                    <span className="field-label">λ selection</span>
                    <span className="field-control">
                      <input type="checkbox" checked={pid.optimize} onChange={(e) => setPid({ optimize: e.target.checked })} />
                      Sweep 0.25 … 4 and keep the best
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
            <div className="group-caption">MPC</div>
            <div className="field-row">
              <NumberField label="Sample time Ts" value={mpc.sampleTime} min={1} max={30} step={1} unit="s" onChange={(v) => setMpc({ sampleTime: v })} />
              <NumberField label="Prediction horizon Np" value={mpc.horizon} min={5} max={120} step={5} unit={`steps = ${mpc.horizon * mpc.sampleTime} s`} onChange={(v) => setMpc({ horizon: v, controlHorizon: Math.min(mpc.controlHorizon, v) })} />
              <NumberField label="Control horizon Nc" value={mpc.controlHorizon} min={1} max={mpc.horizon} step={1} unit={`steps = ${mpc.controlHorizon * mpc.sampleTime} s`} onChange={(v) => setMpc({ controlHorizon: v })} />
              <NumberField label="Move weight λ" value={mpc.moveWeight} min={0} max={10000} step={10} onChange={(v) => setMpc({ moveWeight: v })} />
            </div>
            <p className="muted group-note">
              Optimize は、PID では Kp、Ti、Td を Nelder–Mead 法で、MPC では λ と Nc を候補の掃引で、参照軌道に対する平均絶対誤差が最小になるように探し、この欄に書き戻す。
            </p>
          </fieldset>

          <fieldset className="setup-group">
            <legend>Reference trajectory</legend>
            <div className="field-row">
              <label className="field">
                <span className="field-label">Duration</span>
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
              <NumberField label="Ramp rate (0 = step)" value={scenario.rampRate} min={0} max={60} step={0.5} unit="K/min" onChange={(v) => setScenario({ rampRate: v })} />
            </div>
            <p className="muted group-note">
              目標温度が変わるとき、昇温レートが 0 なら一気に切り替わり、正ならその速さで新しい目標へ移る。開始時は周囲温度から昇温する。
            </p>
          </fieldset>
        </div>

        <PhaseEditor
          heaters={geometry.rows * geometry.cols}
          duration={scenario.duration}
          phases={scenario.phases}
          preset={preset}
          onPreset={(name) => applyPreset(name)}
          onChange={(phases) => {
            setPreset("custom");
            setScenario({ phases });
          }}
        />
      </div>
    </section>
  );
}
