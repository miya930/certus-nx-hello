/** 出力外乱を足した状態空間モデルに対する定常カルマンフィルタ。Python 側の MpcController と同じ構成。 */

import type { Discrete } from "./chain";
import { add, identity, inverse, matmul, matvec, maxAbsDiff, type Matrix, sub, transpose, zeros } from "./linalg";

export interface KalmanSettings {
  stateNoise: number;
  disturbanceNoise: number;
  measurementNoise: number;
  /** 出力外乱の状態を持つか。持たないとモデルのずれが定常偏差として残る。 */
  useDisturbance: boolean;
}

export interface Kalman {
  aAug: Matrix;
  bAug: Matrix;
  cAug: Matrix;
  /** 定常カルマンゲイン L (n_x + n_d) × n_y */
  gain: Matrix;
  /** 予測誤差の共分散 P の対角成分の平方根 [K]。推定の不確かさの目安。 */
  predictedStd: number[];
  iterations: number;
  states: number;
  sensors: number;
}

const MAX_ITERATIONS = 2000;
const TOLERANCE = 1e-10;

/** 離散リッカチ方程式を反復で解き、定常ゲインを求める。 */
export function designKalman(model: Discrete, settings: KalmanSettings): Kalman {
  const nx = model.a.length;
  const ny = model.c.length;
  const nd = settings.useDisturbance ? ny : 0;
  const n = nx + nd;

  const aAug = zeros(n, n);
  model.a.forEach((row, i) => row.forEach((x, j) => (aAug[i][j] = x)));
  for (let i = 0; i < nd; i++) aAug[nx + i][nx + i] = 1;
  const bAug = zeros(n, model.b[0].length);
  model.b.forEach((row, i) => row.forEach((x, j) => (bAug[i][j] = x)));
  const cAug = zeros(ny, n);
  model.c.forEach((row, i) => row.forEach((x, j) => (cAug[i][j] = x)));
  for (let i = 0; i < nd; i++) cAug[i][nx + i] = 1;

  const q = zeros(n, n);
  for (let i = 0; i < nx; i++) q[i][i] = settings.stateNoise;
  for (let i = 0; i < nd; i++) q[nx + i][nx + i] = settings.disturbanceNoise;
  const r = identity(ny).map((row) => row.map((x) => x * settings.measurementNoise));

  // P_{k+1} = A P Aᵀ − A P Cᵀ (C P Cᵀ + R)^-1 C P Aᵀ + Q を収束するまで繰り返す。
  const aT = transpose(aAug);
  const cT = transpose(cAug);
  let p = identity(n);
  let iterations = 0;
  for (; iterations < MAX_ITERATIONS; iterations++) {
    const pcT = matmul(p, cT);
    const s = add(matmul(cAug, pcT), r);
    const gain = matmul(pcT, inverse(s));
    const corrected = sub(p, matmul(gain, matmul(cAug, p)));
    const next = add(matmul(matmul(aAug, corrected), aT), q);
    const change = maxAbsDiff(next, p);
    p = next;
    if (change < TOLERANCE) break;
  }
  const pcT = matmul(p, cT);
  const s = add(matmul(cAug, pcT), r);
  return {
    aAug,
    bAug,
    cAug,
    gain: matmul(pcT, inverse(s)),
    predictedStd: p.map((row, i) => Math.sqrt(Math.max(0, row[i]))),
    iterations,
    states: nx,
    sensors: ny,
  };
}

export interface KalmanStep {
  predicted: number[];
  expected: number[];
  innovation: number[];
  correction: number[];
  updated: number[];
}

/** 1 周期分の予測と更新。途中の値も返し、Notes の計算例に使う。 */
export function kalmanStep(filter: Kalman, estimate: number[], previousInput: number[], measurement: number[]): KalmanStep {
  const predicted = matvec(filter.aAug, estimate).map((x, i) => x + matvec(filter.bAug, previousInput)[i]);
  const expected = matvec(filter.cAug, predicted);
  const innovation = measurement.map((y, i) => y - expected[i]);
  const correction = matvec(filter.gain, innovation);
  const updated = predicted.map((x, i) => x + correction[i]);
  return { predicted, expected, innovation, correction, updated };
}
