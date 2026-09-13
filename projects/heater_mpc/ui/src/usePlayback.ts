import { useEffect, useRef, useState } from "react";

/** シミュレーション上の時刻を、再生速度に合わせて進める。 */
export function usePlayback(duration: number) {
  const [time, setTime] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(60);
  const last = useRef<number | null>(null);

  useEffect(() => {
    setTime((t) => Math.min(t, duration));
  }, [duration]);

  useEffect(() => {
    if (playing && time >= duration) {
      setPlaying(false);
    }
  }, [playing, time, duration]);

  useEffect(() => {
    if (!playing) {
      last.current = null;
      return;
    }
    let frame = 0;
    const tick = (now: number) => {
      const elapsed = last.current === null ? 0 : (now - last.current) / 1000;
      last.current = now;
      setTime((t) => Math.min(duration, t + elapsed * speed));
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [playing, speed, duration]);

  const togglePlaying = () => {
    if (!playing && time >= duration) {
      setTime(0);
    }
    setPlaying(!playing);
  };

  return { time, setTime, playing, togglePlaying, speed, setSpeed };
}
