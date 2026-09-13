import type { ControllerName } from "../api";
import type { ControllerMetrics } from "../metrics";

interface Props {
  name: ControllerName;
  metrics: Record<ControllerName, ControllerMetrics>;
}

const TILES: { key: keyof ControllerMetrics; label: string; unit: string; digits: number }[] = [
  { key: "meanAbsoluteError", label: "Mean |SV − PV|", unit: "K", digits: 2 },
  { key: "worstSensorError", label: "Worst sensor |SV − PV|", unit: "K", digits: 2 },
  { key: "energyWh", label: "Heater energy", unit: "Wh", digits: 2 },
  { key: "peakTemperature", label: "Peak board temperature", unit: "°C", digits: 1 },
];

export function StatTiles({ name, metrics }: Props) {
  const others = (Object.keys(metrics) as ControllerName[]).filter((n) => n !== name);
  return (
    <div className="stat-tiles">
      {TILES.map((tile) => (
        <div key={tile.key} className="stat-tile">
          <div className="stat-label">{tile.label}</div>
          <div className="stat-value">
            {metrics[name][tile.key].toFixed(tile.digits)}
            <span className="stat-unit">{tile.unit}</span>
          </div>
          {others.map((other) => (
            <div key={other} className="stat-compare">
              {other} {metrics[other][tile.key].toFixed(tile.digits)} {tile.unit}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
