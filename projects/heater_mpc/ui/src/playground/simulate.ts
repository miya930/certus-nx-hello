/** Notes のデモの閉ループと開ループのシミュレーション。 */

import { type Chain, discretize, makeChain } from "./chain";
import type { KalmanSettings } from "./kalman";
import { designKalman, type Kalman, kalmanStep } from "./kalman";
import { matvec } from "./linalg";
import { Mpc, type MpcSettings, Pid, type PidGains } from "./controllers";

export const PLANT_DT = 1;
export const DURATION = 600;
export const AMBIENT = 25;

export interface Phase {
  start: number;
  setpoint: number[];
}

/** デモの目標温度。全体 40 °C、200 s で中央だけ 50 °C、400 s で全体 45 °C。 */
export const DEMO_PHASES: Phase[] = [
  { start: 0, setpoint: [40, 40, 40] },
  { start: 200, setpoint: [45, 50, 45] },
  { start: 400, setpoint: [45, 45, 45] },
];

export function referenceTrajectory(phases: Phase[], rampRate: number, samples: number, heaters: number, dt = PLANT_DT): number[][] {
  const reference: number[][] = [];
  let current = new Array<number>(heaters).fill(AMBIENT);
  const maxStep = rampRate <= 0 ? Infinity : (rampRate / 60) * dt;
  for (let i = 0; i < samples; i++) {
    const t = i * dt;
    const target = [...phases].reverse().find((p) => t >= p.start)!.setpoint;
    current = current.map((c, j) => c + Math.min(maxStep, Math.max(-maxStep, target[j] - c)));
    reference.push(current);
  }
  return reference;
}

export interface Trace {
  times: number[];
  sv: number[][];
  pv: number[][];
  mv: number[][];
  meanAbsoluteError: number;
}

interface Controller {
  update(reference: number[] | number[][], measurement: number[]): number[];
  lookahead?: number;
}

function closedLoop(plant: Chain, controller: Controller, controllerDt: number, reference: number[][]): Trace {
  const model = discretize(plant, PLANT_DT);
  const heaters = plant.heaters.length;
  const samples = reference.length;
  const controlSteps = Math.round(controllerDt / PLANT_DT);
  let temperature = new Array<number>(plant.n).fill(0);
  let power = new Array<number>(heaters).fill(0);
  const pv: number[][] = [];
  const mv: number[][] = [];
  let error = 0;
  for (let i = 0; i < samples; i++) {
    const sensor = plant.sensors.map((s) => temperature[s]);
    if (i % controlSteps === 0) {
      const lookahead = controller.lookahead ?? 0;
      const target = lookahead
        ? Array.from({ length: lookahead }, (_, k) => reference[Math.min(samples - 1, i + controlSteps * (k + 1))].map((r) => r - AMBIENT))
        : reference[i].map((r) => r - AMBIENT);
      power = controller.update(target, sensor);
    }
    pv.push(sensor.map((s) => s + AMBIENT));
    mv.push([...power]);
    error += sensor.reduce((s, v, j) => s + Math.abs(v + AMBIENT - reference[i][j]), 0);
    const heat = matvec(model.b, power);
    temperature = matvec(model.a, temperature).map((x, j) => x + heat[j]);
  }
  return {
    times: reference.map((_, i) => i * PLANT_DT),
    sv: reference,
    pv,
    mv,
    meanAbsoluteError: error / (samples * heaters),
  };
}

export function runPid(gains: PidGains, powerMax: number, rampRate: number): Trace {
  const plant = makeChain();
  const reference = referenceTrajectory(DEMO_PHASES, rampRate, DURATION + 1, plant.heaters.length);
  return closedLoop(plant, new Pid(gains, PLANT_DT, powerMax, plant.heaters.length), PLANT_DT, reference);
}

export const DEMO_KALMAN: KalmanSettings = { stateNoise: 1e-4, disturbanceNoise: 1e-2, measurementNoise: 1e-2, useDisturbance: true };

export function runMpc(settings: MpcSettings, sampleTime: number, rampRate: number, modelScale = 1): Trace {
  const plant = makeChain();
  // 予測モデルには、実際の基板とずれたコンダクタンスを使える。
  const model = discretize(makeChain({ conductanceScale: modelScale }), sampleTime);
  const reference = referenceTrajectory(DEMO_PHASES, rampRate, DURATION + 1, plant.heaters.length);
  return closedLoop(plant, new Mpc(model, settings, DEMO_KALMAN), sampleTime, reference);
}

export interface EstimationRun {
  times: number[];
  /** 実際の基板の全格子の温度上昇 [K] */
  truth: number[][];
  /** 推定した全格子の温度上昇 [K]。外乱の推定は含まない。 */
  estimate: number[][];
  /** 補正なしにモデルだけを進めた温度上昇 [K] */
  modelOnly: number[][];
  /** 推定した出力外乱 d̂ [K] */
  disturbance: number[][];
  measurement: number[][];
  input: number[][];
  filter: Kalman;
  /** 各周期の途中の値。計算例の表に使う。 */
  steps: { predicted: number[]; expected: number[]; innovation: number[]; correction: number[]; updated: number[] }[];
}

export interface EstimationSettings extends KalmanSettings {
  sampleTime: number;
  noiseStd: number;
  modelScale: number;
}

/** ヒーターに決まった電力を入れる開ループで、実際の温度とカルマンフィルタの推定を比べる。 */
export function runEstimation(settings: EstimationSettings, seed = 1): EstimationRun {
  const plant = makeChain();
  const plantModel = discretize(plant, PLANT_DT);
  const model = discretize(makeChain({ conductanceScale: settings.modelScale }), settings.sampleTime);
  const filter = designKalman(model, settings);
  const steps = Math.floor(DURATION / settings.sampleTime);
  let random = seed;
  const noise = () => {
    // 決定的な擬似乱数で、スライダーを動かしても同じ雑音の列になるようにする。
    random = (random * 1103515245 + 12345) % 2147483648;
    const u1 = (random + 1) / 2147483649;
    random = (random * 1103515245 + 12345) % 2147483648;
    const u2 = (random + 1) / 2147483649;
    return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
  };
  const inputAt = (t: number) => [0.15, t < 300 ? 0.3 : 0, 0.15];

  let temperature = new Array<number>(plant.n).fill(0);
  let modelOnly = new Array<number>(plant.n).fill(0);
  let estimate = new Array<number>(filter.aAug.length).fill(0);
  let previousInput = new Array<number>(plant.heaters.length).fill(0);
  const run: EstimationRun = {
    times: [],
    truth: [],
    estimate: [],
    modelOnly: [],
    disturbance: [],
    measurement: [],
    input: [],
    filter,
    steps: [],
  };
  for (let k = 0; k <= steps; k++) {
    const t = k * settings.sampleTime;
    const measurement = plant.sensors.map((s) => temperature[s] + settings.noiseStd * noise());
    const step = kalmanStep(filter, estimate, previousInput, measurement);
    estimate = step.updated;
    const heat = matvec(model.b, previousInput);
    modelOnly = matvec(model.a, modelOnly).map((x, i) => x + heat[i]);
    run.times.push(t);
    run.truth.push([...temperature]);
    run.estimate.push(estimate.slice(0, plant.n));
    run.modelOnly.push([...modelOnly]);
    run.disturbance.push(estimate.slice(plant.n));
    run.measurement.push(measurement);
    run.input.push(inputAt(t));
    run.steps.push(step);
    // 実際の基板を制御周期分だけ 1 s 刻みで進める。
    const input = inputAt(t);
    for (let s = 0; s < settings.sampleTime; s++) {
      const q = matvec(plantModel.b, input);
      temperature = matvec(plantModel.a, temperature).map((x, i) => x + q[i]);
    }
    previousInput = input;
  }
  return run;
}
