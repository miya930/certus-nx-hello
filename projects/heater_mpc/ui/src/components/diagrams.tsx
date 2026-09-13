import type { ReactNode } from "react";

/** 図は文字色を継承し、強調したい 1 つの流れだけに accent の色を使う。 */
function Figure({ label, caption, viewBox, children }: { label: string; caption: ReactNode; viewBox: string; children: ReactNode }) {
  return (
    <figure className="diagram">
      <svg viewBox={viewBox} role="img" aria-label={label}>
        {children}
      </svg>
      <figcaption>{caption}</figcaption>
    </figure>
  );
}

function Arrow({ id, accent = false }: { id: string; accent?: boolean }) {
  return (
    <marker id={id} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0,0 L8,4 L0,8 z" className={accent ? "accent-fill" : "ink-fill"} />
    </marker>
  );
}

function Box({ x, y, w, h, title, lines = [], accent = false }: { x: number; y: number; w: number; h: number; title: string; lines?: string[]; accent?: boolean }) {
  return (
    <g>
      <rect x={x} y={y} width={w} height={h} rx={6} className={accent ? "box accent" : "box"} />
      <text x={x + w / 2} y={y + 18} textAnchor="middle" className="title">
        {title}
      </text>
      {lines.map((line, i) => (
        <text key={i} x={x + w / 2} y={y + 38 + i * 16} textAnchor="middle" className="small">
          {line}
        </text>
      ))}
    </g>
  );
}

/** PID のループ。1 つのヒーターの自分のサーミスタだけを使い、隣からの熱は外乱として入る。 */
export function PidLoopDiagram() {
  return (
    <Figure
      label="PID のフィードバックループ。目標値と測定値の差から比例、積分、微分を足し、上下限で切って電力にする。隣のヒーターの熱は基板に外乱として入る。"
      viewBox="0 0 760 260"
      caption="PID のループ。9 個のヒーターにこの図が 9 つあり、それぞれ自分のサーミスタしか見ない。隣のヒーターの熱は上から入る外乱で、積分項が後から打ち消す。"
    >
      <defs>
        <Arrow id="pid-arrow" />
        <Arrow id="pid-arrow-accent" accent />
      </defs>
      <text x={18} y={129} className="title">
        SV r
      </text>
      <line x1={58} y1={125} x2={92} y2={125} className="wire" markerEnd="url(#pid-arrow)" />
      <circle cx={110} cy={125} r={16} className="box" />
      <text x={110} y={130} textAnchor="middle" className="title">
        Σ
      </text>
      <text x={97} y={112} className="small">
        +
      </text>
      <text x={114} y={152} className="small">
        −
      </text>
      <line x1={126} y1={125} x2={196} y2={125} className="wire" markerEnd="url(#pid-arrow)" />
      <text x={162} y={112} textAnchor="middle" className="small">
        e = r − PV
      </text>
      <Box x={198} y={68} w={166} h={114} title="PID" lines={["P: Kp e", "I: Kp/Ti Σ e Ts", "D: −Kp Td ΔPV/Ts"]} />
      <line x1={364} y1={125} x2={388} y2={125} className="wire" markerEnd="url(#pid-arrow)" />
      <Box x={390} y={103} w={70} h={44} title="sat" lines={["0 … umax"]} />
      <line x1={460} y1={125} x2={498} y2={125} className="wire accent" markerEnd="url(#pid-arrow-accent)" />
      <text x={479} y={112} textAnchor="middle" className="small accent-text">
        u [W]
      </text>
      <Box x={500} y={85} w={170} h={80} title="ヒーター + 基板" lines={["1 mm 格子の熱モデル", "自分のサーミスタで PV を読む"]} />
      <line x1={585} y1={30} x2={585} y2={83} className="wire dashed muted-stroke" markerEnd="url(#pid-arrow)" />
      <text x={585} y={20} textAnchor="middle" className="small muted">
        隣のヒーターからの熱 (このループには見えない)
      </text>
      <line x1={670} y1={125} x2={740} y2={125} className="wire" markerEnd="url(#pid-arrow)" />
      <text x={705} y={112} textAnchor="middle" className="small">
        PV [°C]
      </text>
      <polyline points="705,125 705,215 110,215 110,143" className="wire" markerEnd="url(#pid-arrow)" />
      <text x={402} y={232} textAnchor="middle" className="small">
        測定を Ts = 1 s ごとに戻す
      </text>
      <text x={18} y={250} className="small muted">
        × 9 ループ。ループ同士は情報を交換しない。
      </text>
    </Figure>
  );
}

/** MPC が 1 周期に行う演算の流れ。オンラインの 6 段階と、オフラインで済む行列を分ける。 */
export function MpcCycleDiagram() {
  return (
    <Figure
      label="MPC の 1 周期。測定をカルマンフィルタで状態推定に変え、参照軌道の先読みから目標ベクトルを作り、二次計画で操作量の列を求め、先頭だけを基板に加える。A、B、L、Φ、Γ、H はオフラインで一度だけ計算する。"
      viewBox="0 0 900 430"
      caption="MPC の 1 周期 (Ts = 5 s ごと)。上段が毎周期オンラインで行う演算で、左から右へ流れる。QP の解 U は Nc × n_u = 90 個の電力の列で、そのうち先頭の 9 個だけを基板に加える。下段の破線の枠は設定が決まれば一度だけ計算すればよい行列で、オンラインではこれらとベクトルの積を取るだけになる。"
    >
      <defs>
        <Arrow id="mpc-arrow" />
        <Arrow id="mpc-arrow-accent" accent />
      </defs>
      <Box x={20} y={140} w={100} h={80} title="測定" lines={["y_k", "サーミスタ 9 点"]} />
      <line x1={120} y1={180} x2={148} y2={180} className="wire" markerEnd="url(#mpc-arrow)" />
      <Box
        x={150}
        y={120}
        w={190}
        h={120}
        title="カルマンフィルタ"
        lines={["予測 ξ̂ ← A_ξ ξ̂ + B_ξ u_{k−1}", "更新 ξ̂ += L (y_k − C_ξ ξ̂)", "ξ̂ = [x̂ (144), d̂ (9)]"]}
      />
      <line x1={340} y1={180} x2={368} y2={180} className="wire" markerEnd="url(#mpc-arrow)" />
      <text x={354} y={168} textAnchor="middle" className="small">
        x̂, d̂
      </text>
      <Box x={370} y={120} w={180} h={120} title="目標ベクトル" lines={["b_k = [r_{k+i} − d̂]", "      − Φ x̂", "i = 1 … Np (540 個)"]} />
      <line x1={460} y1={40} x2={460} y2={118} className="wire" markerEnd="url(#mpc-arrow)" />
      <text x={460} y={30} textAnchor="middle" className="small">
        参照軌道の先読み r_{"{k+1}"} … r_{"{k+Np}"}
      </text>
      <line x1={550} y1={180} x2={578} y2={180} className="wire" markerEnd="url(#mpc-arrow)" />
      <text x={564} y={168} textAnchor="middle" className="small">
        b_k
      </text>
      <Box x={580} y={120} w={190} h={120} title="二次計画 (QP)" lines={["min ½ UᵀHU + gᵀU", "0 ≤ U ≤ umax", "反復して解く"]} />
      <line x1={770} y1={180} x2={798} y2={180} className="wire" markerEnd="url(#mpc-arrow)" />
      <text x={784} y={168} textAnchor="middle" className="small">
        U
      </text>
      <Box x={800} y={150} w={80} h={60} title="先頭 n_u" lines={["残りは捨てる"]} />
      <line x1={840} y1={210} x2={840} y2={288} className="wire accent" markerEnd="url(#mpc-arrow-accent)" />
      <text x={852} y={252} className="small accent-text">
        u_k (9 個)
      </text>
      <Box x={700} y={290} w={180} h={60} title="基板" lines={["u_k を 5 s 保持"]} accent />
      <polyline points="700,320 70,320 70,222" className="wire" markerEnd="url(#mpc-arrow)" />
      <text x={385} y={336} textAnchor="middle" className="small">
        次の周期の測定 y_{"{k+1}"}
      </text>
      <polyline points="840,262 245,262 245,242" className="wire dashed" markerEnd="url(#mpc-arrow)" />
      <text x={545} y={276} textAnchor="middle" className="small muted">
        前の周期の u_{"{k−1}"} を予測に使う
      </text>
      <rect x={150} y={372} width={620} height={46} rx={6} className="box dashed" />
      <text x={460} y={391} textAnchor="middle" className="title">
        オフラインで一度だけ計算する行列
      </text>
      <text x={460} y={409} textAnchor="middle" className="small">
        A, B (離散化) · L (リッカチ方程式) · Φ, Γ (予測) · H = ΓᵀΓ + λDᵀD (QP のヘッセ行列)
      </text>
      <line x1={245} y1={372} x2={245} y2={244} className="wire dashed muted-stroke" markerEnd="url(#mpc-arrow)" />
      <text x={258} y={300} className="small muted">
        L
      </text>
      <line x1={460} y1={372} x2={460} y2={244} className="wire dashed muted-stroke" markerEnd="url(#mpc-arrow)" />
      <text x={473} y={300} className="small muted">
        Φ
      </text>
      <line x1={675} y1={372} x2={675} y2={244} className="wire dashed muted-stroke" markerEnd="url(#mpc-arrow)" />
      <text x={688} y={300} className="small muted">
        Γ, H
      </text>
    </Figure>
  );
}

/** カルマンフィルタの役割。9 点の測定から 144 個の状態を埋め、モデルのずれを毎周期取り込む。 */
export function KalmanDiagram({ rows, cols, sensorCells }: { rows: number; cols: number; sensorCells: number[] }) {
  const cell = 240 / Math.max(rows, cols);
  const originX = 40;
  const originY = 34;
  const sensors = new Set(sensorCells);
  return (
    <Figure
      label="左は基板の格子で、測れるのは 9 格子だけであり、残りはモデルで推定する。右は時間の流れで、モデルだけの予測は少しずつずれるが、測定との差の一部を毎周期取り込むと推定は現実から離れない。"
      viewBox="0 0 820 320"
      caption="カルマンフィルタの役割。左: MPC の予測に必要な状態は格子全ての温度だが、測れるのは色の付いた格子だけで、残りはモデルで埋める。右: モデルだけで予測を続けるとずれが積もる (破線)。測定 (点) との差にゲイン L をかけて毎周期取り込むと、推定 (実線) は現実から離れない。"
    >
      <defs>
        <Arrow id="kf-arrow" />
      </defs>
      <text x={originX} y={20} className="title">
        {rows * cols} 個の状態、{sensorCells.length} 個の測定
      </text>
      {Array.from({ length: rows * cols }, (_, i) => {
        const r = Math.floor(i / cols);
        const c = i % cols;
        return (
          <rect
            key={i}
            x={originX + c * cell}
            y={originY + r * cell}
            width={cell}
            height={cell}
            className={sensors.has(i) ? "cell accent-fill" : "cell"}
          />
        );
      })}
      <text x={originX} y={originY + rows * cell + 20} className="small">
        色の格子 = サーミスタで測る x[s_i]
      </text>
      <text x={originX} y={originY + rows * cell + 36} className="small muted">
        白い格子 = モデルとゲイン L で推定する
      </text>

      <line x1={400} y1={250} x2={800} y2={250} className="wire" markerEnd="url(#kf-arrow)" />
      <line x1={400} y1={250} x2={400} y2={50} className="wire" markerEnd="url(#kf-arrow)" />
      <text x={790} y={268} textAnchor="end" className="small">
        時間 (制御周期ごと)
      </text>
      <text x={412} y={58} className="small">
        温度
      </text>
      <polyline points="430,200 510,178 590,152 670,124 750,96" className="wire dashed muted-stroke" />
      <polyline points="430,200 510,186 590,172 670,160 750,150" className="wire accent" />
      {[510, 590, 670, 750].map((x, i) => (
        <circle key={x} cx={x} cy={[192, 178, 166, 156][i]} r={4} className="dot" />
      ))}
      <line x1={670} y1={124} x2={670} y2={166} className="wire thin" />
      <text x={682} y={136} className="small">
        y − ŷ (モデルとのずれ)
      </text>
      <line x1={700} y1={196} x2={674} y2={166} className="wire accent thin" markerEnd="url(#kf-arrow)" />
      <text x={704} y={212} className="small accent-text">
        L × ずれ を推定に足す
      </text>
      <text x={470} y={92} className="small muted">
        破線: モデルだけの予測
      </text>
      <text x={470} y={108} className="small">
        点: 測定 (9 点)
      </text>
      <text x={470} y={124} className="small accent-text">
        実線: 推定 x̂, d̂
      </text>
      <text x={400} y={300} className="small muted">
        d̂ は「モデルで説明できない温度差」をためる状態で、PID の積分項に相当する。
      </text>
    </Figure>
  );
}

/** 移動ホライズン。予測区間の先まで計画し、先頭だけを加え、次の周期に 1 ステップずらしてやり直す。 */
export function RecedingHorizonDiagram() {
  return (
    <Figure
      label="時間軸の上に、今の時刻から予測ホライズン先までの参照軌道と予測した温度、制御ホライズン分の計画した電力を描いた図。計画のうち最初の 1 ステップだけを加え、次の周期に 1 ステップずらして同じ計算をやり直す。"
      viewBox="0 0 820 300"
      caption="移動ホライズン。今 k から Np ステップ先までの参照軌道を見て、Nc ステップ分の電力の列を計画し、その先は最後の値を保持すると仮定する。加えるのは色を付けた最初の 1 ステップだけで、次の周期には全体を 1 ステップずらして計算し直す。"
    >
      <defs>
        <Arrow id="rh-arrow" />
      </defs>
      <rect x={40} y={50} width={160} height={200} className="past" />
      <text x={120} y={44} textAnchor="middle" className="small muted">
        過去
      </text>
      <line x1={40} y1={250} x2={800} y2={250} className="wire" markerEnd="url(#rh-arrow)" />
      <line x1={200} y1={50} x2={200} y2={250} className="wire thin" />
      <text x={200} y={268} textAnchor="middle" className="small">
        今 k
      </text>
      <line x1={380} y1={200} x2={380} y2={250} className="wire thin dashed" />
      <text x={380} y={268} textAnchor="middle" className="small">
        k + Nc
      </text>
      <line x1={700} y1={60} x2={700} y2={250} className="wire thin dashed" />
      <text x={700} y={268} textAnchor="middle" className="small">
        k + Np
      </text>
      <line x1={200} y1={30} x2={700} y2={30} className="wire thin" markerStart="url(#rh-arrow)" markerEnd="url(#rh-arrow)" />
      <text x={450} y={24} textAnchor="middle" className="small">
        予測ホライズン Np (300 s)
      </text>
      <polyline points="40,150 200,150 380,90 800,90" className="wire dashed muted-stroke" />
      <text x={735} y={82} className="small muted">
        参照軌道 r
      </text>
      <polyline points="200,172 260,148 320,118 380,100 470,92 560,90 700,90" className="wire" />
      <text x={300} y={140} className="small">
        予測した PV
      </text>
      <text x={48} y={214} className="small">
        計画した電力 U
      </text>
      <rect x={200} y={196} width={45} height={40} className="bar accent-fill" />
      <rect x={245} y={204} width={45} height={32} className="bar" />
      <rect x={290} y={212} width={45} height={24} className="bar" />
      <rect x={335} y={218} width={45} height={18} className="bar" />
      <rect x={380} y={218} width={320} height={18} className="bar hold" />
      <text x={222} y={190} textAnchor="middle" className="small accent-text">
        加える u_k
      </text>
      <text x={540} y={212} textAnchor="middle" className="small muted">
        最後の値を保持
      </text>
      <text x={450} y={292} textAnchor="middle" className="small">
        次の周期 k + 1 では、全体を 1 ステップ右にずらして同じ計算をやり直す →
      </text>
    </Figure>
  );
}
