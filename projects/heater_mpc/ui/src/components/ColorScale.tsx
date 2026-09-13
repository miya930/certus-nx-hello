import { heatGradient } from "../heatColor";

interface Props {
  domain: [number, number];
}

const TICKS = 5;

export function ColorScale({ domain }: Props) {
  const values = Array.from({ length: TICKS }, (_, i) => domain[0] + ((domain[1] - domain[0]) * i) / (TICKS - 1));
  return (
    <div className="color-scale" aria-label={`Temperature scale from ${domain[0]} to ${domain[1]} °C`}>
      <div className="color-scale-bar" style={{ background: heatGradient() }} />
      <div className="color-scale-ticks">
        {values.map((v) => (
          <span key={v}>{v.toFixed(0)}</span>
        ))}
      </div>
      <div className="color-scale-unit">°C</div>
    </div>
  );
}
