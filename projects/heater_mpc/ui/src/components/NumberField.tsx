interface Props {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit?: string;
  onChange: (value: number) => void;
}

export function NumberField({ label, value, min, max, step, unit, onChange }: Props) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <span className="field-control">
        <input
          type="number"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(event) => {
            // 入力の途中で空になった瞬間は値を変えない。
            const next = event.target.valueAsNumber;
            if (!Number.isNaN(next)) {
              onChange(next);
            }
          }}
        />
        {unit && <span className="muted">{unit}</span>}
      </span>
    </label>
  );
}
