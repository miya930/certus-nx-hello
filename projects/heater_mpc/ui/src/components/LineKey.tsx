interface Props {
  color: string;
  dash?: string;
}

const WIDTH = 22;
const HEIGHT = 8;

export function LineKey({ color, dash }: Props) {
  return (
    <svg className="line-key" width={WIDTH} height={HEIGHT} aria-hidden="true">
      <line x1={0} x2={WIDTH} y1={HEIGHT / 2} y2={HEIGHT / 2} stroke={color} strokeWidth={2} strokeDasharray={dash} />
    </svg>
  );
}
