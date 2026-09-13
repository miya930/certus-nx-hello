import { useCallback, useEffect, useMemo, useState } from "react";
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
import { Notes, NOTES_SECTIONS } from "./components/Notes";
import { OptimizationCard } from "./components/OptimizationCard";
import { SetupPanel } from "./components/SetupPanel";
import { type Page, Sidebar } from "./components/Sidebar";
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
const CONTROLLERS: ControllerName[] = ["PID", "MPC"];

export function App() {
  const [page, setPage] = useState<Page>("simulation");
  const [pendingSection, setPendingSection] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<string | null>(null);
  const [request, setRequest] = useState(DEFAULT_REQUEST);
  const [result, setResult] = useState<SimulationResult | null>(null);
  const [optimization, setOptimization] = useState<OptimizationResult | null>(null);
  const [running, setRunning] = useState(false);
  const [optimizing, setOptimizing] = useState(false);
  const [progress, setProgress] = useState<JobProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [setupOpen, setSetupOpen] = useState(true);
  const [controller, setController] = useState<ControllerName>("PID");
  const [hovered, setHovered] = useState<number | null>(null);
  const [selected, setSelected] = useState(4);
  const playback = usePlayback(result ? (result.sv.length - 1) * result.timeStep : DEFAULT_DURATION);

  // ノートのページに切り替わってから、選んだ節までスクロールする。
  useEffect(() => {
    if (page === "notes" && pendingSection) {
      document.getElementById(pendingSection)?.scrollIntoView({ behavior: "smooth", block: "start" });
      setActiveSection(pendingSection);
      setPendingSection(null);
    }
  }, [page, pendingSection]);

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
        // 見つかった値を設定に書き戻し、次の実行で使えるようにする。
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
  const statusText = running ? "計算中" : optimizing ? "最適化中" : error ? "エラー" : result ? "結果あり" : "待機";
  const status = <span className={`status ${busy ? "running" : error ? "error" : result ? "ready" : ""}`}>{statusText}</span>;

  return (
    <div className="shell">
      <Sidebar
        page={page}
        sections={NOTES_SECTIONS}
        activeSection={activeSection}
        status={status}
        onPage={(p) => {
          setPage(p);
          if (p === "notes" && !pendingSection) setPendingSection(NOTES_SECTIONS[0].id);
        }}
        onSection={(id) => {
          setPage("notes");
          setPendingSection(id);
        }}
      />
      <div className="content">
        {page === "simulation" ? (
          <>
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

            {error && <div className="error-banner">失敗しました: {error}</div>}

            {optimization && <OptimizationCard result={optimization} onClose={() => setOptimization(null)} />}

            {result && metrics ? (
              <main className={running ? "stale" : ""}>
                <div className="view-switch" role="tablist" aria-label="制御器">
                  {CONTROLLERS.map((name) => (
                    <button key={name} type="button" role="tab" aria-selected={controller === name} className={controller === name ? "active" : ""} onClick={() => setController(name)}>
                      {name} の結果
                    </button>
                  ))}
                </div>
                <ControllerView
                  result={result}
                  name={controller}
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
                {busy
                  ? `${progress?.stage ?? (running ? "計算中" : "パラメータを探索中")} … ${Math.round((progress?.fraction ?? 0) * 100)} %`
                  : "基板と参照軌道を決めて、「シミュレーションを実行」を押すと、ここに結果が出る。"}
              </div>
            )}
          </>
        ) : (
          <main>
            <Notes result={result} />
          </main>
        )}
      </div>
    </div>
  );
}
