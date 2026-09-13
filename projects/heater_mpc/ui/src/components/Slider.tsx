interface Props {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  /** 桁が広い量は、指数を動かす対数のスライダーにする。 */
  log?: boolean;
  digits?: number;
  onChange: (value: number) => void;
}

const LOG_STEPS = 60;

export function Slider({ label, value, min, max, step, unit, log = false, digits = 2, onChange }: Props) {
  const position = log ? (Math.log10(value) - Math.log10(min)) / (Math.log10(max) - Math.log10(min)) : value;
  const shown = log ? (value >= 100 || value < 0.01 ? value.toExponential(0) : value.toPrecision(2)) : value.toFixed(digits);
  return (
    <label className="slider">
      <span className="slider-head">
        <span>{label}</span>
        <span className="mono">
          {shown}
          {unit ? ` ${unit}` : ""}
        </span>
      </span>
      <input
        type="range"
        min={log ? 0 : min}
        max={log ? 1 : max}
        step={log ? 1 / LOG_STEPS : step}
        value={position}
        onChange={(e) => {
          const p = Number(e.target.value);
          onChange(log ? 10 ** (Math.log10(min) + p * (Math.log10(max) - Math.log10(min))) : p);
        }}
      />
    </label>
  );
}
