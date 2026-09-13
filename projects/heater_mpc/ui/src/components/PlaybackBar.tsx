interface Props {
  time: number;
  duration: number;
  playing: boolean;
  speed: number;
  phaseLabel: string;
  onToggle: () => void;
  onSeek: (time: number) => void;
  onSpeed: (speed: number) => void;
}

const SPEEDS = [15, 30, 60, 120];

export function PlaybackBar(props: Props) {
  return (
    <div className="playback">
      <button type="button" className="play-button" onClick={props.onToggle} aria-label={props.playing ? "Pause" : "Play"}>
        {props.playing ? "❚❚" : "▶"}
      </button>
      <div className="playback-readout">
        <span className="mono">t = {props.time.toFixed(0).padStart(4, "0")} s</span>
        <span className="muted">/ {props.duration.toFixed(0)} s</span>
      </div>
      <input
        className="scrubber"
        type="range"
        min={0}
        max={props.duration}
        step={1}
        value={props.time}
        onChange={(event) => props.onSeek(Number(event.target.value))}
        aria-label="Simulation time"
      />
      <div className="segmented" role="group" aria-label="Playback speed">
        {SPEEDS.map((s) => (
          <button
            key={s}
            type="button"
            className={s === props.speed ? "active" : ""}
            onClick={() => props.onSpeed(s)}
          >
            ×{s}
          </button>
        ))}
      </div>
      <span className="chip">{props.phaseLabel}</span>
    </div>
  );
}
