import { useMemo } from "react";
import type { Scenario } from "../api";
import { referenceTrajectory } from "../playground/simulate";
import { seriesStyle } from "../series";
import { MiniChart } from "./MiniChart";

interface Props {
  scenario: Scenario;
  heaters: number;
}

const STEP = 5;

/** Setup で決めた段階と昇温レートから、参照軌道をその場で描く。 */
export function ReferencePreview({ scenario, heaters }: Props) {
  const { times, series } = useMemo(() => {
    const samples = Math.floor(scenario.duration / STEP) + 1;
    const phases = scenario.phases.map((p) => ({ start: p.start, setpoint: p.setpoint }));
    const reference = referenceTrajectory(phases, scenario.rampRate, samples, heaters, STEP);
    return {
      times: reference.map((_, i) => i * STEP),
      series: Array.from({ length: heaters }, (_, h) => ({
        label: seriesStyle(h).label,
        values: reference.map((row) => row[h]),
        color: seriesStyle(h).color,
      })),
    };
  }, [scenario, heaters]);
  return <MiniChart title="参照軌道 SV (全ヒーター)" unit="°C" times={times} series={series} height={170} />;
}
