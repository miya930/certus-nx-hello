import type { ControllerResult, SimulationResult } from "./api";

export interface ControllerMetrics {
  meanAbsoluteError: number;
  worstSensorError: number;
  energyWh: number;
  peakTemperature: number;
}

export function controllerMetrics(
  result: SimulationResult,
  controller: ControllerResult<unknown>,
): ControllerMetrics {
  const heaters = controller.pv[0].length;
  const sensorError = new Array<number>(heaters).fill(0);
  let energy = 0;
  controller.pv.forEach((row, step) => {
    const sv = result.sv[step];
    row.forEach((value, i) => {
      sensorError[i] += Math.abs(value - sv[i]);
      energy += controller.mv[step][i] * result.timeStep;
    });
  });
  const samples = controller.pv.length;
  const peak = Math.max(...controller.frames.map((frame) => Math.max(...frame)));
  return {
    meanAbsoluteError: sensorError.reduce((a, b) => a + b, 0) / (samples * heaters),
    worstSensorError: Math.max(...sensorError) / samples,
    energyWh: energy / 3600,
    peakTemperature: peak / result.frame.scale,
  };
}

/** 行が時刻、列がヒーターの配列を、ヒーターごとの時系列に並べ替える。 */
export function byHeater(rows: number[][]): number[][] {
  return rows[0].map((_, i) => rows.map((row) => row[i]));
}
