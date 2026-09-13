/** Notes のデモで使う PID と MPC。Python 側の pid_controller と mpc_controller と同じ式。 */

import type { Discrete } from "./chain";
import { designKalman, type Kalman, type KalmanSettings, kalmanStep } from "./kalman";
import { largestEigenvalue, matmul, matvec, type Matrix, transpose, zeros } from "./linalg";

const DERIVATIVE_FILTER = 10;

export interface PidGains {
  kp: number;
  ti: number;
  td: number;
}

export class Pid {
  private integral: number[];
  private derivative: number[];
  private previous: number[] | null = null;

  constructor(
    private gains: PidGains,
    private dt: number,
    private powerMax: number,
    heaters: number,
  ) {
    this.integral = new Array<number>(heaters).fill(0);
    this.derivative = new Array<number>(heaters).fill(0);
  }

  update(setpoint: number[], measurement: number[]): number[] {
    const { kp, ti, td } = this.gains;
    const error = setpoint.map((r, i) => r - measurement[i]);
    if (this.previous) {
      const tf = td / DERIVATIVE_FILTER;
      const slope = measurement.map((y, i) => (y - this.previous![i]) / this.dt);
      this.derivative = this.derivative.map((d, i) => d + (this.dt / (tf + this.dt)) * (-td * slope[i] - d));
    }
    this.previous = measurement;
    const unclamped = error.map((e, i) => kp * (e + this.integral[i] + this.derivative[i]));
    const power = unclamped.map((u) => Math.min(this.powerMax, Math.max(0, u)));
    // 出力が飽和している向きには積分を進めず、ワインドアップを防ぐ。
    this.integral = this.integral.map((v, i) => {
      const saturated = unclamped[i] !== power[i] && Math.sign(error[i]) === Math.sign(unclamped[i] - power[i]);
      return saturated ? v : v + (error[i] * this.dt) / ti;
    });
    return power;
  }
}

export interface MpcSettings {
  horizon: number;
  controlHorizon: number;
  moveWeight: number;
  powerMax: number;
}

const QP_ITERATIONS = 300;

export class Mpc {
  readonly filter: Kalman;
  readonly lookahead: number;
  private free: Matrix;
  private forced: Matrix;
  private hessian: Matrix;
  private forcedT: Matrix;
  private difference: Matrix;
  private step: number;
  /** 対角前処理の倍率 sqrt(H_ii)。列ごとの大きさの差をならして、勾配法の収束を速める。 */
  private scaling: number[];
  estimate: number[];
  power: number[];
  private plan: number[];
  lastInnovation: number[] = [];

  constructor(
    private model: Discrete,
    private settings: MpcSettings,
    kalman: KalmanSettings,
  ) {
    const heaters = model.b[0].length;
    this.filter = designKalman(model, kalman);
    this.lookahead = settings.horizon;
    this.free = this.freeResponse();
    this.forced = this.forcedResponse();
    this.forcedT = transpose(this.forced);
    const moves = settings.controlHorizon * heaters;
    this.difference = zeros(moves, moves);
    for (let i = 0; i < moves; i++) {
      this.difference[i][i] = 1;
      if (i >= heaters) this.difference[i][i - heaters] = -1;
    }
    const dT = transpose(this.difference);
    const h = matmul(this.forcedT, this.forced);
    const dd = matmul(dT, this.difference);
    const hessian = h.map((row, i) => row.map((x, j) => x + settings.moveWeight * dd[i][j]));
    this.scaling = hessian.map((row, i) => Math.sqrt(row[i]));
    this.hessian = hessian.map((row, i) => row.map((x, j) => x / (this.scaling[i] * this.scaling[j])));
    this.step = 1 / largestEigenvalue(this.hessian);
    this.estimate = new Array<number>(this.filter.aAug.length).fill(0);
    this.power = new Array<number>(heaters).fill(0);
    this.plan = new Array<number>(moves).fill(0);
  }

  private freeResponse(): Matrix {
    const rows: number[][] = [];
    let propagate = this.model.c;
    for (let i = 0; i < this.settings.horizon; i++) {
      propagate = matmul(propagate, this.model.a);
      rows.push(...propagate);
    }
    return rows;
  }

  private forcedResponse(): Matrix {
    const { a, b, c } = this.model;
    const states = a.length;
    const heaters = b[0].length;
    const columns: number[][] = [];
    for (let move = 0; move < this.settings.controlHorizon; move++) {
      for (let heater = 0; heater < heaters; heater++) {
        let x = new Array<number>(states).fill(0);
        const outputs: number[] = [];
        for (let step = 0; step < this.settings.horizon; step++) {
          const held = step === move || (move === this.settings.controlHorizon - 1 && step > move);
          x = matvec(a, x).map((v, i) => v + (held ? b[i][heater] : 0));
          outputs.push(...matvec(c, x));
        }
        columns.push(outputs);
      }
    }
    return transpose(columns);
  }

  /** reference は行が予測ホライズンの各ステップ、列がセンサー。 */
  update(reference: number[][], measurement: number[]): number[] {
    const step = kalmanStep(this.filter, this.estimate, this.power, measurement);
    this.estimate = step.updated;
    this.lastInnovation = step.innovation;
    const states = this.filter.states;
    const disturbance = this.estimate.slice(states);
    const x = this.estimate.slice(0, states);
    const freeY = matvec(this.free, x);
    const target = reference.flatMap((row) => row.map((r, i) => r - (disturbance[i] ?? 0))).map((t, i) => t - freeY[i]);

    // g = −Γᵀ b − λ Dᵀ U_prev。前の周期の解をずらした値から反復を始める。
    const heaters = this.power.length;
    const previous = new Array<number>(this.plan.length).fill(0);
    this.power.forEach((u, i) => (previous[i] = u));
    const gForced = matvec(this.forcedT, target).map((v) => -v);
    const gMove = matvec(transpose(this.difference), previous).map((v) => -this.settings.moveWeight * v);
    const g = gForced.map((v, i) => v + gMove[i]);
    const warm = [...this.plan.slice(heaters), ...this.plan.slice(-heaters)];
    this.plan = this.projectedGradient(g, warm);
    this.power = this.plan.slice(0, heaters);
    return this.power;
  }

  /** 加速付き射影勾配法で 0 ≤ U ≤ umax の二次計画を解く。変数は sqrt(H_ii) で前処理した v = D U で扱う。 */
  private projectedGradient(g: number[], start: number[]): number[] {
    const d = this.scaling;
    const gScaled = g.map((v, i) => v / d[i]);
    const clip = (v: number, i: number) => Math.min(this.settings.powerMax * d[i], Math.max(0, v));
    let u = start.map((v, i) => clip(v * d[i], i));
    let y = [...u];
    let t = 1;
    for (let k = 0; k < QP_ITERATIONS; k++) {
      const grad = matvec(this.hessian, y).map((v, i) => v + gScaled[i]);
      const next = y.map((v, i) => clip(v - this.step * grad[i], i));
      const tNext = (1 + Math.sqrt(1 + 4 * t * t)) / 2;
      y = next.map((v, i) => v + ((t - 1) / tNext) * (v - u[i]));
      u = next;
      t = tNext;
    }
    return u.map((v, i) => v / d[i]);
  }
}
