import { useMemo } from "react";
import type { ControllerName, SimulationResult } from "../api";
import { byHeater, type ControllerMetrics } from "../metrics";
import { BoardHeatmap } from "./BoardHeatmap";
import { ColorScale } from "./ColorScale";
import { ControllerParameters } from "./ControllerParameters";
import { LoopPanel } from "./LoopPanel";
import { OverlayChart } from "./OverlayChart";
import { PlaybackBar } from "./PlaybackBar";
import { SensorGrid } from "./SensorGrid";
import { StatTiles } from "./StatTiles";

interface Props {
  result: SimulationResult;
  name: ControllerName;
  metrics: Record<ControllerName, ControllerMetrics>;
  domain: [number, number];
  playback: {
    time: number;
    playing: boolean;
    speed: number;
    togglePlaying: () => void;
    setTime: (time: number) => void;
    setSpeed: (speed: number) => void;
  };
  hovered: number | null;
  selected: number;
  onHover: (index: number | null) => void;
  onSelect: (index: number) => void;
}

const DESCRIPTIONS: Record<ControllerName, string> = {
  PID: "ヒーターごとに独立した PID ループ。各ヒーターは自分のサーミスタしか見ない。",
  MPC: "1 つのモデル予測制御器が、参照軌道に沿って全ヒーターの電力をまとめて計画する。",
};

export function ControllerView(props: Props) {
  const { result, name, playback } = props;
  const controller = result.controllers[name];
  const pvSeries = useMemo(() => byHeater(controller.pv), [controller]);
  const mvSeries = useMemo(() => byHeater(controller.mv), [controller]);
  const phaseStarts = useMemo(() => result.phases.map((p) => p.start), [result]);
  const duration = (result.sv.length - 1) * result.timeStep;

  const step = Math.min(controller.pv.length - 1, Math.round(playback.time / result.timeStep));
  const frameIndex = Math.min(controller.frames.length - 1, Math.floor(playback.time / result.frame.interval));
  const pv = controller.pv[step];
  const mv = controller.mv[step];
  const sv = result.sv[step];
  const phaseIndex = result.phases.reduce((found, p, i) => (playback.time >= p.start ? i : found), 0);

  return (
    <div className="controller-view">
      <p className="view-description">{DESCRIPTIONS[name]}</p>
      <div className="view-top">
        <section className="card">
          <header className="card-header">
            <h2>基板の温度分布</h2>
            <ColorScale domain={props.domain} />
          </header>
          <BoardHeatmap
            result={result}
            frame={controller.frames[frameIndex]}
            domain={props.domain}
            pv={pv}
            sv={sv}
            mv={mv}
            hovered={props.hovered}
            selected={props.selected}
            onHover={props.onHover}
            onSelect={props.onSelect}
          />
          <PlaybackBar
            time={playback.time}
            duration={duration}
            playing={playback.playing}
            speed={playback.speed}
            phaseLabel={`段階 ${phaseIndex + 1} / ${result.phases.length}`}
            onToggle={playback.togglePlaying}
            onSeek={playback.setTime}
            onSpeed={playback.setSpeed}
          />
        </section>
        <div className="view-side">
          <StatTiles name={name} metrics={props.metrics} />
          <section className="card">
            <header className="card-header">
              <h2>{name} のパラメータ</h2>
            </header>
            <ControllerParameters name={name} result={result} />
          </section>
        </div>
      </div>

      <section className="card">
        <header className="card-header">
          <h2>
            サーミスタの温度 <span className="muted">全ヒーターの PV</span>
          </h2>
        </header>
        <div className="overlay-layout">
          <OverlayChart
            label="全サーミスタの温度"
            unit="°C"
            digits={1}
            cols={result.cols}
            timeStep={result.timeStep}
            values={pvSeries}
            phaseStarts={phaseStarts}
            time={playback.time}
            highlight={props.hovered}
            onSeek={playback.setTime}
          />
          <SensorGrid
            cols={result.cols}
            pv={pv}
            sv={sv}
            mv={mv}
            hovered={props.hovered}
            selected={props.selected}
            onHover={props.onHover}
            onSelect={props.onSelect}
          />
        </div>
      </section>

      <LoopPanel
        result={result}
        name={name}
        selected={props.selected}
        onSelect={props.onSelect}
        time={playback.time}
        onSeek={playback.setTime}
      />

      <section className="card">
        <header className="card-header">
          <h2>
            ヒーターの電力 <span className="muted">全ヒーターの MV</span>
          </h2>
        </header>
        <OverlayChart
          label="全ヒーターの電力"
          unit="W"
          digits={3}
          cols={result.cols}
          timeStep={result.timeStep}
          values={mvSeries}
          phaseStarts={phaseStarts}
          time={playback.time}
          highlight={props.hovered}
          onSeek={playback.setTime}
          includeZero
        />
      </section>
    </div>
  );
}
