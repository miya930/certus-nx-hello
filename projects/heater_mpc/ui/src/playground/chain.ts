/** Notes のデモで使う、1 次元に格子を並べた基板の熱モデル。物性値は Python 側と同じ FR4 と銅箔の値。 */

import { identity, inverse, matmul, type Matrix, zeros } from "./linalg";

const K_FR4 = 0.3;
const RHO_FR4 = 1850;
const CP_FR4 = 1100;
const K_CU = 385;
const RHO_CU = 8960;
const CP_CU = 385;
const T_CU = 35e-6;
const COPPER_LAYERS = 2;
const H_SIDE = 10;

export interface Chain {
  n: number;
  dx: number;
  heaters: number[];
  sensors: number[];
  /** 格子 1 つの熱容量 [J/K] */
  capacity: number;
  /** 隣の格子への熱コンダクタンス [W/K] */
  conductance: number;
  /** 両面からの放熱のコンダクタンス [W/K] */
  loss: number;
}

export interface ChainOptions {
  cells?: number;
  dxMm?: number;
  thicknessMm?: number;
  copper?: number;
  heaters?: number[];
  sensors?: number[];
  /** モデルのずれを作るために、熱コンダクタンスに掛ける倍率 */
  conductanceScale?: number;
}

export function makeChain(options: ChainOptions = {}): Chain {
  const { cells = 15, dxMm = 5, thicknessMm = 1.6, copper = 0.5, heaters = [2, 7, 12], sensors = [3, 8, 13], conductanceScale = 1 } = options;
  const dx = dxMm * 1e-3;
  const tCu = T_CU * COPPER_LAYERS * copper;
  const tFr4 = thicknessMm * 1e-3 - T_CU * COPPER_LAYERS;
  return {
    n: cells,
    dx,
    heaters,
    sensors,
    capacity: (RHO_FR4 * CP_FR4 * tFr4 + RHO_CU * CP_CU * tCu) * dx * dx,
    conductance: (K_FR4 * tFr4 + K_CU * tCu) * conductanceScale,
    loss: 2 * H_SIDE * dx * dx,
  };
}

export interface Discrete {
  a: Matrix;
  b: Matrix;
  c: Matrix;
  dt: number;
}

/** 後退オイラー法で離散化する。Python 側の Board.discrete_model と同じ式。 */
export function discretize(chain: Chain, dt: number): Discrete {
  const { n, capacity, conductance, loss } = chain;
  const k = zeros(n, n);
  for (let i = 0; i < n; i++) {
    const neighbors = (i > 0 ? 1 : 0) + (i < n - 1 ? 1 : 0);
    k[i][i] = neighbors * conductance + loss;
    if (i > 0) k[i][i - 1] = -conductance;
    if (i < n - 1) k[i][i + 1] = -conductance;
  }
  const storage = capacity / dt;
  const system = k.map((row, i) => row.map((x, j) => x + (i === j ? storage : 0)));
  const inv = inverse(system);
  const a = inv.map((row) => row.map((x) => x * storage));
  const q = zeros(n, chain.heaters.length);
  chain.heaters.forEach((cell, j) => (q[cell][j] = 1));
  const b = matmul(inv, q);
  const c = zeros(chain.sensors.length, n);
  chain.sensors.forEach((cell, i) => (c[i][cell] = 1));
  return { a, b, c, dt };
}

/** 定常ゲイン行列 K_ss = C K^-1 Q [K/W]。 */
export function steadyGain(chain: Chain): Matrix {
  const { n, conductance, loss } = chain;
  const k = zeros(n, n);
  for (let i = 0; i < n; i++) {
    const neighbors = (i > 0 ? 1 : 0) + (i < n - 1 ? 1 : 0);
    k[i][i] = neighbors * conductance + loss;
    if (i > 0) k[i][i - 1] = -conductance;
    if (i < n - 1) k[i][i + 1] = -conductance;
  }
  const inv = inverse(k);
  return chain.sensors.map((s) => chain.heaters.map((h) => inv[s][h]));
}

export { identity };
