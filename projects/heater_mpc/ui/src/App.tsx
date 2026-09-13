import { useCallback, useMemo, useState } from "react";
import {
  type ControllerName,
  fetchOptimization,
  fetchSimulation,
  type JobProgress,
  type OptimizationResult,
  type OptimizeTarget,
  type SimulationRequest,
  type SimulationResult,
} from "./api";
import { ControllerView } from "./components/ControllerView";
import { Notes } from "./components/Notes";
import { OptimizationCard } from "./components/OptimizationCard";
import { SetupPanel } from "./components/SetupPanel";
import { Tabs } from "./components/Tabs";
import { controllerMetrics } from "./metrics";
import { presetPhases } from "./presets";
import { usePlayback } from "./usePlayback";

const DEFAULT_DURATION = 1800;
const DEFAULT_REQUEST: SimulationRequest = {
  geometry: { rows: 3, cols: 3, pitchMm: 20, marginMm: 10, heaterXMm: 6, heaterYMm: 3, sensorOffsetMm: 3, thicknessMm: 1.6, copper: 0.5 },
  powerMax: 0.5,
  scenario: { duration: DEFAULT_DURATION, rampRate: 0, phases: presetPhases("steps", 3, 3, DEFAULT_DURATION) },
  pid: { mode: "imc", lambdaRatio: 0.5, optimize: false, kp: 0.065, ti: 59, td: 0.5 },
  mpc: { sampleTime: 5, horizon: 60, controlHorizon: 10, moveWeight: 100 },
};
const TABS = ["PID", "MPC", "Notes"] as const;
type Tab = (typeof TABS)[number];

export function App() {
  const [request, setRequest] = useState(DEFAULT_REQUEST);
  const [result, setResult] = useState<SimulationResult | null>(null);
  const [optimization, setOptimization] = useState<OptimizationResult | null>(null);
  const [running, setRunning] = useState(false);
  const [optimizing, setOptimizing] = useState(false);
  const [progress, setProgress] = useState<JobProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [setupOpen, setSetupOpen] = useState(true);
  const [tab, setTab] = useState<Tab>("PID");
  const [hovered, setHovered] = useState<number | null>(null);
  const [selected, setSelected] = useState(4);
  const playback = usePlayback(result ? (result.sv.length - 1) * result.timeStep : DEFAULT_DURATION);

  const run = useCallback(async () => {
    setRunning(true);
    setError(null);
    try {
      const next = await fetchSimulation(request, setProgress);
      setResult(next);
      const center = Math.floor(next.rows / 2) * next.cols + Math.floor(next.cols / 2);
      setSelected((s) => (s < next.rows * next.cols ? s : center));
      setSetupOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setRunning(false);
      setProgress(null);
    }
  }, [request]);

  const optimize = useCallback(
    async (target: OptimizeTarget) => {
      setOptimizing(true);
      setError(null);
      try {
        const found = await fetchOptimization(request, target, setProgress);
        setOptimization(found);
        // 見つかった値を Setup に書き戻し、次の Run で使えるようにする。
        setRequest((current) => ({
          ...current,
          pid: found.pid ? { ...current.pid, mode: "explicit", kp: found.pid.kp, ti: found.pid.ti, td: found.pid.td } : current.pid,
          mpc: found.mpc ? { ...current.mpc, moveWeight: found.mpc.moveWeight, controlHorizon: found.mpc.controlHorizon } : current.mpc,
        }));
        setSetupOpen(true);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setOptimizing(false);
        setProgress(null);
      }
    },
    [request],
  );

  const metrics = useMemo(
    () =>
      result && {
        PID: controllerMetrics(result, result.controllers.PID),
        MPC: controllerMetrics(result, result.controllers.MPC),
      },
    [result],
  );

  // 2 つの制御の温度分布を同じ色で比べられるよう、色の範囲を共通にする。
  const domain = useMemo<[number, number]>(() => {
    if (!metrics || !result) {
      return [0, 1];
    }
    return [result.ambient, Math.ceil(Math.max(metrics.PID.peakTemperature, metrics.MPC.peakTemperature))];
  }, [metrics, result]);

  const busy = running || optimizing;
  const status = running ? "Simulating" : optimizing ? "Optimizing" : error ? "Error" : result ? "Ready" : "Idle";

  return (
    <div className="app">
      <header className="app-header">
        <div>
          <h1>Heater MPC Simulator</h1>
          <p className="muted">Chip heaters and thermistors on one FR4 board — PID vs model predictive control</p>
        </div>
        <span className={`status ${busy ? "running" : error ? "error" : result ? "ready" : ""}`}>{status}</span>
      </header>

      <SetupPanel
        request={request}
        running={running}
        optimizing={optimizing}
        progress={progress}
        open={setupOpen}
        onToggle={() => setSetupOpen((o) => !o)}
        onChange={setRequest}
        onRun={run}
        onOptimize={optimize}
      />

      {error && <div className="error-banner">Request failed: {error}</div>}

      {optimization && <OptimizationCard result={optimization} onClose={() => setOptimization(null)} />}

      <Tabs tabs={TABS} active={tab} onChange={setTab} />

      {tab === "Notes" ? (
        <main>
          <Notes result={result} />
        </main>
      ) : result && metrics ? (
        <main className={running ? "stale" : ""}>
          <ControllerView
            result={result}
            name={tab as ControllerName}
            metrics={metrics}
            domain={domain}
            playback={playback}
            hovered={hovered}
            selected={selected}
            onHover={setHovered}
            onSelect={setSelected}
          />
        </main>
      ) : (
        <div className="placeholder">
          {running || optimizing
            ? `${progress?.stage ?? (running ? "Running the simulation" : "Searching controller parameters")} … ${Math.round((progress?.fraction ?? 0) * 100)} %`
            : "Set up the board and the reference trajectory, then press Run simulation."}
        </div>
      )}
    </div>
  );
}
