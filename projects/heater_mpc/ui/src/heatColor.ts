// 温度は熱の配色で表す。明るさが単調に増える inferno の代表点を線形補間する。
const STOPS: [number, number, number][] = [
  [0, 0, 4],
  [27, 12, 65],
  [74, 12, 107],
  [120, 28, 109],
  [165, 44, 96],
  [207, 68, 70],
  [237, 105, 37],
  [251, 155, 6],
  [252, 255, 164],
];

export function heatColor(ratio: number): [number, number, number] {
  const clamped = Math.min(1, Math.max(0, ratio));
  const position = clamped * (STOPS.length - 1);
  const lower = Math.min(STOPS.length - 2, Math.floor(position));
  const fraction = position - lower;
  const [a, b] = [STOPS[lower], STOPS[lower + 1]];
  return [0, 1, 2].map((i) => Math.round(a[i] + (b[i] - a[i]) * fraction)) as [number, number, number];
}

export function heatGradient(): string {
  const stops = STOPS.map(([r, g, b], i) => `rgb(${r} ${g} ${b}) ${(i / (STOPS.length - 1)) * 100}%`);
  return `linear-gradient(to right, ${stops.join(", ")})`;
}
