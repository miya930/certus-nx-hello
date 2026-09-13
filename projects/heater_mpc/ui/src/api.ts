export interface Geometry {
  rows: number;
  cols: number;
  pitchMm: number;
  marginMm: number;
  heaterXMm: number;
  heaterYMm: number;
  sensorOffsetMm: number;
  thicknessMm: number;
  copper: number;
}

export interface Phase {
  start: number;
  setpoint: number[];
}

export interface Scenario {
  duration: number;
  rampRate: number;
  phases: Phase[];
}

export type PidMode = "imc" | "explicit";

export interface PidSettings {
  mode: PidMode;
  lambdaRatio: number;
  optimize: boolean;
  kp: number;
  ti: number;
  td: number;
}

export interface MpcSettings {
  sampleTime: number;
  horizon: number;
  controlHorizon: number;
  moveWeight: number;
}

export interface SimulationRequest {
  geometry: Geometry;
  powerMax: number;
  scenario: Scenario;
  pid: PidSettings;
  mpc: MpcSettings;
}

export interface Point {
  x: number;
  y: number;
}

export interface ResultPhase extends Phase {
  requiredPower: number[];
}

export interface Fopdt {
  gain: number;
  tau: number;
  delay: number;
}

export interface PidInfo {
  mode: PidMode;
  sampleTime: number;
  kp: number;
  ti: number;
  td: number;
  derivativeFilter: number;
  fopdt: Fopdt;
  lambdaRatio?: number;
  closedLoopTime?: number;
  optimized?: boolean;
  sweep?: { lambdaRatio: number; meanAbsoluteError: number }[];
}

export interface MpcInfo extends MpcSettings {
  modelDxMm: number;
  states: number;
  stateNoise: number;
  disturbanceNoise: number;
  measurementNoise: number;
}

/** MPC が予測に使う離散時間の状態空間モデル。行列は行優先の配列。 */
export interface StateSpaceModel {
  dxMm: number;
  sampleTime: number;
  rows: number;
  cols: number;
  states: number;
  cellCapacity: number;
  conductance: number;
  cellLoss: number;
  heaterCells: number[];
  sensorCells: number[];
  a: number[][];
  b: number[][];
  timeConstants: number[];
}

export interface ControllerResult<Info> {
  info: Info;
  pv: number[][];
  mv: number[][];
  frames: number[][];
  meanAbsoluteError: number;
}

export interface SimulationResult {
  ambient: number;
  timeStep: number;
  powerMax: number;
  rows: number;
  cols: number;
  board: { widthMm: number; heightMm: number; heaterSizeMm: Point; heaters: Point[]; sensors: Point[] };
  frame: { interval: number; scale: number; rows: number; cols: number; cellMm: number };
  sv: number[][];
  rampRate: number;
  phases: ResultPhase[];
  plant: {
    dxMm: number;
    cells: number;
    conductance: number;
    arealCapacity: number;
    cellCapacity: number;
    cellLoss: number;
    convection: number;
    characteristicLengthMm: number;
    steadyGain: number[][];
  };
  controllers: { PID: ControllerResult<PidInfo>; MPC: ControllerResult<MpcInfo> & { model: StateSpaceModel } };
}

export type ControllerName = keyof SimulationResult["controllers"];

export type OptimizeTarget = "both" | "pid" | "mpc";

export interface PidGains {
  kp: number;
  ti: number;
  td: number;
}

export interface PidOptimization extends PidGains {
  evaluations: number;
  searchError: number;
  plantError: number;
  start: PidGains & { plantError: number };
  history: (PidGains & { error: number })[];
}

export interface MpcTrial {
  moveWeight: number;
  controlHorizon: number;
  error: number;
}

export interface MpcOptimization {
  moveWeight: number;
  controlHorizon: number;
  plantError: number;
  start: MpcTrial & { plantError: number };
  trials: MpcTrial[];
}

export interface OptimizationResult {
  pid?: PidOptimization;
  mpc?: MpcOptimization;
  seconds: number;
}

export interface JobProgress {
  stage: string;
  fraction: number;
}

interface JobSnapshot<T> {
  status: "running" | "done" | "error";
  stage: string;
  fraction: number;
  result?: T;
  error?: string;
}

const POLL_INTERVAL_MS = 250;

async function readError(response: Response) {
  let message = `${response.status} ${response.statusText}`;
  try {
    const payload = (await response.json()) as { error?: string };
    if (payload.error) {
      message = payload.error;
    }
  } catch {
    // 本文が JSON でなければ、状態行をそのまま使う。
  }
  return new Error(message);
}

/** 計算をジョブとして始め、終わるまで進み具合を問い合わせ続ける。 */
async function runJob<T>(path: string, body: unknown, onProgress: (progress: JobProgress) => void): Promise<T> {
  const started = await fetch(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!started.ok) {
    throw await readError(started);
  }
  const { job } = (await started.json()) as { job: string };
  for (;;) {
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    const response = await fetch(`/api/jobs/${job}`);
    if (!response.ok) {
      throw await readError(response);
    }
    const snapshot = (await response.json()) as JobSnapshot<T>;
    onProgress({ stage: snapshot.stage, fraction: snapshot.fraction });
    if (snapshot.status === "done") {
      return snapshot.result as T;
    }
    if (snapshot.status === "error") {
      throw new Error(snapshot.error ?? "simulation failed");
    }
  }
}

export function fetchSimulation(request: SimulationRequest, onProgress: (p: JobProgress) => void) {
  return runJob<SimulationResult>("/api/simulate", request, onProgress);
}

export function fetchOptimization(request: SimulationRequest, target: OptimizeTarget, onProgress: (p: JobProgress) => void) {
  return runJob<OptimizationResult>("/api/optimize", { ...request, target }, onProgress);
}
