import type { Phase } from "../api";
import { PRESETS, type PresetName } from "../presets";
import { seriesStyle } from "../series";
import { LineKey } from "./LineKey";

interface Props {
  heaters: number;
  duration: number;
  phases: Phase[];
  preset: PresetName;
  onPreset: (name: PresetName) => void;
  onChange: (phases: Phase[]) => void;
}

const MAX_PHASES = 8;
const SETPOINT_MIN = 25;
const SETPOINT_MAX = 150;
const NEW_PHASE_GAP = 300;

export function PhaseEditor({ heaters, duration, phases, preset, onPreset, onChange }: Props) {
  const update = (index: number, change: Partial<Phase>) =>
    onChange(phases.map((phase, i) => (i === index ? { ...phase, ...change } : phase)));

  const setValue = (index: number, heater: number, value: number) => {
    const setpoint = phases[index].setpoint.map((v, i) => (i === heater ? value : v));
    update(index, { setpoint });
  };

  const add = () => {
    const last = phases[phases.length - 1];
    const start = Math.min(duration - 1, last.start + NEW_PHASE_GAP);
    onChange([...phases, { start, setpoint: [...last.setpoint] }]);
  };

  const remove = (index: number) => onChange(phases.filter((_, i) => i !== index));

  const read = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.valueAsNumber;
    return Number.isNaN(value) ? null : value;
  };

  return (
    <div className="phase-editor">
      <div className="phase-toolbar">
        <label className="inline-field">
          プリセット
          <select value={preset} onChange={(event) => onPreset(event.target.value as PresetName)}>
            {PRESETS.map((p) => (
              <option key={p.name} value={p.name}>
                {p.label}
              </option>
            ))}
          </select>
        </label>
        <span className="muted preset-description">{PRESETS.find((p) => p.name === preset)?.description}</span>
        <button type="button" className="ghost-button" onClick={add} disabled={phases.length >= MAX_PHASES}>
          + 段階を追加
        </button>
      </div>
      <div className="table-scroll">
        <table className="phase-table">
          <thead>
            <tr>
              <th>段階</th>
              <th>開始 [s]</th>
              {Array.from({ length: heaters }, (_, i) => (
                <th key={i}>
                  <LineKey color={seriesStyle(i).color} /> {seriesStyle(i).label}
                </th>
              ))}
              <th />
            </tr>
          </thead>
          <tbody>
            {phases.map((phase, index) => (
              <tr key={index}>
                <td className="muted">{index + 1}</td>
                <td>
                  <input
                    type="number"
                    min={0}
                    max={duration}
                    step={10}
                    value={phase.start}
                    disabled={index === 0}
                    onChange={(event) => {
                      const value = read(event);
                      if (value !== null) update(index, { start: Math.min(duration, Math.max(0, value)) });
                    }}
                  />
                </td>
                {phase.setpoint.map((value, heater) => (
                  <td key={heater}>
                    <input
                      type="number"
                      min={SETPOINT_MIN}
                      max={SETPOINT_MAX}
                      step={1}
                      value={value}
                      onChange={(event) => {
                        const next = read(event);
                        if (next !== null) setValue(index, heater, next);
                      }}
                    />
                  </td>
                ))}
                <td>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={() => remove(index)}
                    disabled={phases.length === 1}
                    aria-label={`Remove phase ${index + 1}`}
                  >
                    ×
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="muted phase-hint">
        目標温度は °C で、{SETPOINT_MIN} … {SETPOINT_MAX} の範囲。開始時刻は最初の段階が 0 s に固定される。
      </div>
    </div>
  );
}
