import type { Phase } from "./api";

export type PresetName = "steps" | "neighbors" | "checker" | "ramp" | "custom";

export const PRESETS: { name: PresetName; label: string; description: string }[] = [
  { name: "steps", label: "Steps: uniform → center → rows", description: "全体 40 °C、中央だけ 50 °C、行ごとに 50 / 45 / 40 °C の順に切り替える。" },
  { name: "neighbors", label: "Neighbors heat the center", description: "周囲のヒーターを 50 °C に上げ、中央だけ 40 °C に保つ。中央は隣からの入熱を捨てる手段がないため、最も厳しい条件になる。" },
  { name: "checker", label: "Checkerboard", description: "市松模様に 47 °C と 43 °C を並べ、隣同士で逆向きの制御を行う。" },
  { name: "ramp", label: "Single ramp to 60 °C", description: "全体を 60 °C にする 1 段階だけ。昇温レートと組み合わせて使う。" },
  { name: "custom", label: "Custom", description: "表を直接編集した目標温度。" },
];

const UNIFORM = 40;

function pattern(rows: number, cols: number, value: (row: number, col: number) => number): number[] {
  return Array.from({ length: rows * cols }, (_, i) => value(Math.floor(i / cols), i % cols));
}

export function presetPhases(name: PresetName, rows: number, cols: number, duration: number): Phase[] {
  const centerRow = Math.floor(rows / 2);
  const centerCol = Math.floor(cols / 2);
  const isCenter = (r: number, c: number) => r === centerRow && c === centerCol;
  const uniform = (value: number) => pattern(rows, cols, () => value);
  const thirds = [0, duration / 3, (2 * duration) / 3];
  switch (name) {
    case "steps":
      return [
        { start: thirds[0], setpoint: uniform(UNIFORM) },
        { start: thirds[1], setpoint: pattern(rows, cols, (r, c) => (isCenter(r, c) ? 50 : 45)) },
        { start: thirds[2], setpoint: pattern(rows, cols, (r) => (rows === 1 ? 45 : 50 - (10 * r) / (rows - 1))) },
      ];
    case "neighbors":
      return [
        { start: thirds[0], setpoint: uniform(UNIFORM) },
        { start: thirds[1], setpoint: pattern(rows, cols, (r, c) => (isCenter(r, c) ? UNIFORM : 50)) },
        { start: thirds[2], setpoint: uniform(UNIFORM) },
      ];
    case "checker":
      return [
        { start: thirds[0], setpoint: uniform(UNIFORM) },
        { start: thirds[1], setpoint: pattern(rows, cols, (r, c) => ((r + c) % 2 === 0 ? 47 : 43)) },
        { start: thirds[2], setpoint: uniform(45) },
      ];
    case "ramp":
      return [{ start: 0, setpoint: uniform(60) }];
    case "custom":
      return [{ start: 0, setpoint: uniform(UNIFORM) }];
  }
}

/** ヒーターの数が変わったときに、既存の目標温度を新しい数にそろえる。 */
export function resizeSetpoints(phases: Phase[], heaters: number): Phase[] {
  return phases.map((phase) => ({
    ...phase,
    setpoint: Array.from({ length: heaters }, (_, i) => phase.setpoint[i] ?? phase.setpoint.at(-1) ?? UNIFORM),
  }));
}
