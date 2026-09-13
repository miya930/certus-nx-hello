import type { ReactNode } from "react";
import type { SimulationResult } from "../api";
import { seriesStyle } from "../series";
import { KalmanDiagram, MpcCycleDiagram, PidLoopDiagram, RecedingHorizonDiagram } from "./diagrams";
import { KalmanPlayground } from "./KalmanPlayground";
import { MpcPlayground } from "./MpcPlayground";
import { PidPlayground } from "./PidPlayground";
import { ModelValues } from "./ModelValues";
import { Tex } from "./Tex";

interface Props {
  result: SimulationResult | null;
}

function MatrixTable({ values, digits, unit }: { values: number[][]; digits: number; unit: string }) {
  return (
    <div className="table-scroll">
      <table className="data-table">
        <thead>
          <tr>
            <th>
              <span className="muted">センサー \ ヒーター</span>
            </th>
            {values[0].map((_, j) => (
              <th key={j}>{seriesStyle(j).label}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {values.map((row, i) => (
            <tr key={i}>
              <th>{seriesStyle(i).label}</th>
              {row.map((v, j) => (
                <td key={j} className={i === j ? "diagonal" : ""}>
                  {v.toFixed(digits)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      <div className="muted table-unit">単位: {unit}</div>
    </div>
  );
}

/** 結果があるときだけ中身を出し、ないときはその旨を 1 行で示す。 */
function WithResult({ result, children }: { result: SimulationResult | null; children: (r: SimulationResult) => ReactNode }) {
  if (!result) {
    return <p className="muted values-placeholder">この基板での値は、Run simulation を実行すると表示される。</p>;
  }
  return <div className="values">{children(result)}</div>;
}

export const NOTES_SECTIONS = [
  { id: "notes-overview", title: "1. シミュレータの構成" },
  { id: "notes-plant", title: "2. プラントモデル" },
  { id: "notes-model-values", title: "2b. 状態空間モデルの値" },
  { id: "notes-reference", title: "3. 参照軌道" },
  { id: "notes-pid", title: "4. PID 制御" },
  { id: "notes-mpc", title: "5. モデル予測制御 (MPC)" },
  { id: "notes-kalman", title: "6. カルマンフィルタの計算例" },
];

export function Notes({ result }: Props) {
  const heaters = result ? result.rows * result.cols : null;
  const mpc = result?.controllers.MPC.info;
  const pid = result?.controllers.PID.info;

  return (
    <article className="notes">
      <section className="card" id="notes-overview">
        <h2>1. シミュレータの構成</h2>
        <p>FR4 の基板に、チップ抵抗のヒーターとサーミスタを格子に並べる。2 層の銅箔が各層の一部を覆うとし、厚さと銅箔の割合は Setup で決める。</p>
        <p>実際の基板の役は 1 mm 格子の熱モデルが担い、PID 制御と MPC はサーミスタの温度だけを見てヒーターの電力を決める。</p>
        <p>温度は周囲温度 25 °C からの上昇として計算し、表示のときに周囲温度を足す。</p>
        <WithResult result={result}>
          {(r) => (
            <p>
              今の基板は {r.board.widthMm.toFixed(0)} × {r.board.heightMm.toFixed(0)} mm で、ヒーターは {r.rows} 行 {r.cols} 列、
              間隔は {r.board.heaters.length > 1 ? `${(r.board.heaters[1].x - r.board.heaters[0].x).toFixed(0)} mm` : "—"}、
              最大電力は {r.powerMax.toFixed(2)} W である。
            </p>
          )}
        </WithResult>
      </section>

      <section className="card" id="notes-plant">
        <h2>2. プラントモデル</h2>
        <h3>熱方程式</h3>
        <p>薄い板の面内の熱伝導と、両面からの放熱を考える。</p>
        <Tex block math={String.raw`C'' \frac{\partial T}{\partial t} = \nabla \cdot \left( G \, \nabla T \right) - 2h \, T + q`} />
        <p>
          <Tex math={String.raw`G = k_\mathrm{FR4} t_\mathrm{FR4} + k_\mathrm{Cu} t_\mathrm{Cu}`} /> は面内の熱コンダクタンス、
          <Tex math={String.raw`C'' = \rho_\mathrm{FR4} c_\mathrm{FR4} t_\mathrm{FR4} + \rho_\mathrm{Cu} c_\mathrm{Cu} t_\mathrm{Cu}`} /> は面積あたりの熱容量である。
        </p>
        <p>
          <Tex math="h" /> は片面あたりの熱伝達率で、自然対流と放射をまとめて 10 W/(m²·K) とした。
          熱が届く距離の目安は特性長 <Tex math={String.raw`L = \sqrt{G / 2h}`} /> である。
        </p>
        <WithResult result={result}>
          {(r) => (
            <p>
              今の基板では <Tex math={`G = ${r.plant.conductance.toFixed(4)}`} /> W/K、<Tex math={`C'' = ${r.plant.arealCapacity.toFixed(0)}`} /> J/(m²·K)、
              <Tex math={`L = ${r.plant.characteristicLengthMm.toFixed(1)}`} /> mm である。
            </p>
          )}
        </WithResult>

        <h3>状態空間モデル</h3>
        <p>
          基板を一辺 <Tex math={String.raw`\Delta x`} /> の正方形の格子に分け、各格子の温度を状態 <Tex math="x" /> とする。
        </p>
        <Tex block math={String.raw`\mathbf{C} \, \dot{x} = -\mathbf{K} x + \mathbf{Q} u, \qquad y = \mathbf{C}_y x`} />
        <p>
          <Tex math={String.raw`\mathbf{C} = C'' \Delta x^2 \, I`} /> は熱容量、
          <Tex math={String.raw`\mathbf{K} = G \, \mathbf{L}_\mathrm{grid} + 2h \Delta x^2 I`} /> は隣の格子への伝導と両面からの放熱、
          <Tex math={String.raw`\mathbf{Q}`} /> はヒーターの電力を格子に配る行列、
          <Tex math={String.raw`\mathbf{C}_y`} /> はサーミスタの位置の格子を取り出す行列である。
        </p>
        <p>時間は後退オイラー法で離散化する。</p>
        <Tex
          block
          math={String.raw`x_{k+1} = A x_k + B u_k, \quad A = \left(\tfrac{\mathbf{C}}{\Delta t} + \mathbf{K}\right)^{-1} \tfrac{\mathbf{C}}{\Delta t}, \quad B = \left(\tfrac{\mathbf{C}}{\Delta t} + \mathbf{K}\right)^{-1} \mathbf{Q}`}
        />
        <p>
          実際の基板の役のモデルは <Tex math={String.raw`\Delta x = 1`} /> mm、<Tex math={String.raw`\Delta t = 1`} /> s で、
          入力と出力の数はどちらもヒーターの数である。
        </p>
        <WithResult result={result}>
          {(r) => (
            <p>
              今の基板では状態の数が <Tex math={`n = ${r.plant.cells}`} />、入力と出力の数が {r.rows * r.cols} である。
            </p>
          )}
        </WithResult>

        <h3>定常ゲイン行列</h3>
        <p>
          一定の電力を入れ続けたときの温度上昇は <Tex math={String.raw`y_\infty = K_\mathrm{ss} u`} />、
          <Tex math={String.raw`K_\mathrm{ss} = \mathbf{C}_y \mathbf{K}^{-1} \mathbf{Q}`} /> で表される。
        </p>
        <p>対角成分は自分のヒーターによる温度上昇で、対角以外の成分がヒーター同士の干渉である。</p>
        <WithResult result={result}>{(r) => <MatrixTable values={r.plant.steadyGain} digits={1} unit="K/W" />}</WithResult>
      </section>

      <section className="card" id="notes-model-values">
        <h2>2b. 状態空間モデルの値</h2>
        <p>
          1 mm 格子のモデルは状態が多く表示には大きすぎるため、同じ構造で MPC が予測に使う 5 mm 格子のモデルの <Tex math="A" />、<Tex math="B" />、
          <Tex math="C" /> の値を示す。
        </p>
        <WithResult result={result}>
          {(r) => (
            <>
              <p>
                1 mm 格子では、格子 1 つの熱容量が {r.plant.cellCapacity.toPrecision(4)} J/K、放熱のコンダクタンスが {r.plant.cellLoss.toPrecision(4)} W/K になり、
                隣への熱コンダクタンス <Tex math="G" /> は格子の大きさによらず {r.plant.conductance.toFixed(4)} W/K である。
              </p>
              <ModelValues result={r} />
            </>
          )}
        </WithResult>
      </section>

      <section className="card" id="notes-reference">
        <h2>3. 参照軌道</h2>
        <p>目標温度は段階ごとに与え、各段階の開始時刻に切り替える。</p>
        <p>
          昇温レート <Tex math="R" /> [K/min] を正にすると、参照軌道は周囲温度から始まり、目標が変わるたびに 1 ステップあたり{" "}
          <Tex math={String.raw`R \, \Delta t / 60`} /> を上限にして新しい目標へ近づく。
        </p>
        <Tex block math={String.raw`r_{k+1} = r_k + \operatorname{clip}\left( r^\ast_{k+1} - r_k, \; -\tfrac{R \Delta t}{60}, \; \tfrac{R \Delta t}{60} \right)`} />
        <p>各段階で目標温度を定常的に実現するのに必要な電力は、定常ゲイン行列を解いて求める。</p>
        <p>必要な電力が 0 W から最大電力の範囲を外れる段階は、どの制御でも目標温度に届かない。隣からの入熱を捨てる手段がないためである。</p>
        <WithResult result={result}>
          {(r) => (
            <>
              <p>このシミュレーションの昇温レートは {r.rampRate > 0 ? `${r.rampRate} K/min` : "0 で、目標は一気に切り替わる"}。</p>
              <div className="table-scroll">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>段階</th>
                      <th>開始</th>
                      <th />
                      {Array.from({ length: r.rows * r.cols }, (_, i) => (
                        <th key={i}>{seriesStyle(i).label}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {r.phases.flatMap((phase, index) => [
                      <tr key={`${index}-sv`}>
                        <td rowSpan={2}>{index + 1}</td>
                        <td rowSpan={2}>{phase.start.toFixed(0)} s</td>
                        <td>目標 (°C)</td>
                        {phase.setpoint.map((v, i) => (
                          <td key={i}>{v.toFixed(0)}</td>
                        ))}
                      </tr>,
                      <tr key={`${index}-power`}>
                        <td className="muted">必要な電力 (W)</td>
                        {phase.requiredPower.map((v, i) => (
                          <td key={i} className={v < 0 || v > r.powerMax ? "out-of-range" : ""}>
                            {v.toFixed(3)}
                          </td>
                        ))}
                      </tr>,
                    ])}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </WithResult>
      </section>

      <section className="card" id="notes-pid">
        <h2>4. PID 制御</h2>
        <h3>教科書的な解説</h3>
        <p>PID 制御は、目標値と測定値の偏差 <Tex math="e = r - y" /> から操作量を決める。</p>
        <Tex block math={String.raw`u(t) = K_p \left( e(t) + \frac{1}{T_i} \int_0^t e(\tau) \, d\tau + T_d \frac{de(t)}{dt} \right)`} />
        <p>比例動作は、今の偏差に比例して操作量を変える。</p>
        <p>積分動作は、偏差を積み上げて、比例動作だけでは残る定常偏差をなくす。</p>
        <p>微分動作は、偏差の変化の速さに応じて操作量を変え、応答の行き過ぎを抑える。</p>
        <p>伝達関数で書くと、微分に一次のフィルタを付けた次の形になる。</p>
        <Tex block math={String.raw`C(s) = K_p \left( 1 + \frac{1}{T_i s} + \frac{T_d s}{1 + (T_d / N) s} \right)`} />

        <h3>このシミュレーションでの実装</h3>
        <PidLoopDiagram />
        <p>各ヒーターを、自分のサーミスタだけを見る独立したループで制御する。</p>
        <p>
          図の上から入る隣のヒーターの熱は、このループには見えない。PV が上がって初めて偏差として現れ、積分項が時間をかけて電力を下げることで打ち消す。
          複数のループが同時に動くと、互いの熱を外乱として打ち消し合うため、落ち着くまでに時間がかかる。
        </p>
        <p>微分は偏差ではなく測定値にかけ、目標値を変えた瞬間に操作量が跳ねないようにした。</p>
        <p>
          操作量は <Tex math={String.raw`0 \le u \le u_\mathrm{max}`} /> に制限し、飽和している向きには積分を止めてワインドアップを防ぐ。
        </p>
        <Tex
          block
          math={String.raw`\begin{aligned} I_k &= I_{k-1} + \frac{T_s}{T_i} e_k \\ D_k &= D_{k-1} + \frac{T_s}{T_f + T_s} \left( -T_d \frac{y_k - y_{k-1}}{T_s} - D_{k-1} \right), \quad T_f = \frac{T_d}{N} \\ u_k &= \operatorname{sat}\left( K_p (e_k + I_k + D_k) \right) \end{aligned}`}
        />

        <h3>調整に使う伝達関数モデル</h3>
        <p>中央のヒーターに 1 W のステップを入れたときの、自分のサーミスタの応答を一次遅れとむだ時間で近似する。</p>
        <Tex block math={String.raw`G(s) = \frac{K e^{-\theta s}}{\tau s + 1}`} />
        <p>
          <Tex math="\tau" /> と <Tex math="\theta" /> は、最終値の 28 % と 63 % に達する時刻から求める。
        </p>
        <Tex block math={String.raw`\tau = 1.5 \, (t_{63} - t_{28}), \qquad \theta = t_{63} - \tau`} />
        <p>
          IMC の調整則は、閉ループの時定数 <Tex math="\lambda" /> を決めると PID のパラメータを与える。
        </p>
        <Tex
          block
          math={String.raw`K_p = \frac{2\tau + \theta}{K (2\lambda + \theta)}, \quad T_i = \tau + \frac{\theta}{2}, \quad T_d = \frac{\tau \theta}{2\tau + \theta}`}
        />
        <p>
          <Tex math="\lambda" /> を小さくすると速く追従するが、隣からの入熱に対して振動しやすくなる。
          IMC の式は最適化ではなく、1 つのループを単独で見たときの目安を与える調整則である。
        </p>
        <p>
          Setup の Optimize は、IMC の値を初期値にして、2 mm 格子の基板モデル上で参照軌道に対する平均絶対誤差が最小になるように Nelder–Mead 法で{" "}
          <Tex math="K_p" />、<Tex math="T_i" />、<Tex math="T_d" /> を探索し、1 mm の基板で確かめる。評価関数は誤差だけなので、操作量の変化の大きさは抑えていない。
        </p>
        <WithResult result={result}>
          {(r) => {
            const p = r.controllers.PID.info;
            return (
              <>
                <p>今の基板の近似は次のとおりである。</p>
                <Tex
                  block
                  math={String.raw`G(s) = \frac{${p.fopdt.gain.toFixed(2)} \, e^{-${p.fopdt.delay.toFixed(1)} s}}{${p.fopdt.tau.toFixed(1)} s + 1} \ \ [\mathrm{K/W}]`}
                />
                {p.mode === "imc" ? (
                  <>
                    <p>
                      PID のパラメータは <Tex math={String.raw`\lambda = ${(p.lambdaRatio ?? 0).toFixed(2)} \, \tau = ${(p.closedLoopTime ?? 0).toFixed(1)}`} /> s
                      とした IMC の式で決めた。
                    </p>
                    <Tex block math={String.raw`K_p = ${p.kp.toFixed(4)}, \quad T_i = ${p.ti.toFixed(1)}, \quad T_d = ${p.td.toFixed(2)}`} />
                    {p.optimized && p.sweep ? (
                      <>
                        <p>
                          <Tex math={String.raw`\lambda / \tau`} /> の候補を全て試し、参照軌道に対する平均絶対誤差が最小になるものを選んだ。
                        </p>
                        <table className="data-table">
                          <thead>
                            <tr>
                              <th>λ / τ</th>
                              {p.sweep.map((s) => (
                                <th key={s.lambdaRatio}>{s.lambdaRatio}</th>
                              ))}
                            </tr>
                          </thead>
                          <tbody>
                            <tr>
                              <td>平均 |SV − PV| (K)</td>
                              {p.sweep.map((s) => (
                                <td key={s.lambdaRatio} className={s.lambdaRatio === p.lambdaRatio ? "diagonal" : ""}>
                                  {s.meanAbsoluteError.toFixed(3)}
                                </td>
                              ))}
                            </tr>
                          </tbody>
                        </table>
                      </>
                    ) : (
                      <p>
                        <Tex math={String.raw`\lambda / \tau`} /> は Setup で決めた値に固定している。
                      </p>
                    )}
                  </>
                ) : (
                  <p>
                    <Tex math="K_p" />、<Tex math="T_i" />、<Tex math="T_d" /> は Setup で直接与えた値{" "}
                    <Tex math={`${p.kp.toFixed(4)}, \\ ${p.ti.toFixed(1)}, \\ ${p.td.toFixed(2)}`} /> で、上の近似はゲインの計算には使っていない。
                  </p>
                )}
              </>
            );
          }}
        </WithResult>
        <h3>この構成の限界</h3>
        <p>各ループは定常ゲイン行列の対角成分だけを前提にしており、隣のヒーターの熱は外乱として後から打ち消すしかない。</p>
        <p>そのため、複数の目標値を同時に変えると、互いの熱で行き過ぎたり、落ち着くまで時間がかかったりする。</p>

        <h3>デモ: ゲインを動かして応答を見る</h3>
        <p>
          サーバーを使わず、ブラウザの中で 1 次元の小さな模型を計算する。15 個の格子を 5 mm 間隔で並べ、25 mm 間隔の 3 個のヒーターと、その隣の格子のサーミスタを置いた基板で、
          目標は全体 40 °C、200 s で中央だけ 50 °C、400 s で全体 45 °C に変わる。スライダーを動かすと 600 s 分を計算し直す。
        </p>
        <PidPlayground />
      </section>

      <section className="card" id="notes-mpc">
        <h2>5. モデル予測制御 (MPC)</h2>
        <h3>教科書的な解説</h3>
        <p>MPC は、制御周期ごとに次の手順を繰り返す。</p>
        <ol>
          <li>モデルで、今の状態から予測ホライズン <Tex math="N_p" /> ステップ先までの出力を予測する。</li>
          <li>
            制約を守りながら評価関数を最小にする、制御ホライズン <Tex math="N_c" /> ステップ分の操作量を求める。
          </li>
          <li>求めた操作量のうち最初の 1 ステップ分だけを加え、次の周期で予測をやり直す。</li>
        </ol>
        <p>予測する区間が周期ごとに先へずれていくため、この考え方を移動ホライズン (receding horizon) と呼ぶ。</p>
        <RecedingHorizonDiagram />
        <p>多入力多出力の干渉と、操作量の上下限を、最適化の中で直接扱えるのが特長である。</p>
        <p>
          PID との違いは 2 つある。1 つは、全ヒーターの電力を 1 つの最適化でまとめて決めるので、隣のヒーターの熱が届くことを前もって知っていることである。
          もう 1 つは、参照軌道の先を見ているので、目標が変わる前に電力を動かし始められることである。
        </p>

        <h3>予測に使う状態空間モデル</h3>
        <p>
          実際の基板の役より粗い <Tex math={String.raw`\Delta x = 5`} /> mm 格子のモデルを、制御周期 <Tex math="T_s" /> で離散化して使う。
          ヒーターとサーミスタの位置は、この格子に丸めて置く。
        </p>
        <p>モデルの誤差を吸収するため、出力に一定の外乱 <Tex math="d" /> が加わると考える。</p>
        <Tex
          block
          math={String.raw`\begin{aligned} x_{k+1} &= A x_k + B u_k, & x &\in \mathbb{R}^{n_x}, \ u \in \mathbb{R}^{n_u} \\ d_{k+1} &= d_k, & d &\in \mathbb{R}^{n_y} \\ y_k &= C x_k + d_k, & y &\in \mathbb{R}^{n_y} \end{aligned}`}
        />

        <h3>状態推定: カルマンフィルタがあると何が嬉しいか</h3>
        <p>MPC の予測は、今の基板全体の温度 <Tex math="x" /> から始める。しかし測れるのはサーミスタの点だけで、残りの格子の温度は分からない。</p>
        <p>カルマンフィルタは、この測定と、前の周期に加えた電力、そしてモデルから、全ての格子の温度を推定する。これが 1 つ目の役割である。</p>
        <p>
          2 つ目の役割は、モデルのずれを毎周期取り込むことである。予測モデルは 5 mm 格子で実際の基板より粗く、ヒーターの位置も丸めている。
          モデルだけで予測を続けるとずれが積もるが、測定との差の一部を毎周期推定に足すと、推定は現実から離れない。
        </p>
        <p>
          3 つ目の役割は、定常偏差をなくすことである。出力外乱 <Tex math="d" /> は「モデルで説明できない温度差」をためる状態で、PID の積分項に相当する。
          目標ベクトルを作るときに <Tex math={String.raw`\hat{d}`} /> を引くので、モデルが少し違っていても最終的な温度は目標に一致する。
        </p>
        {result ? (
          <KalmanDiagram
            rows={result.controllers.MPC.model.rows}
            cols={result.controllers.MPC.model.cols}
            sensorCells={result.controllers.MPC.model.sensorCells}
          />
        ) : (
          <KalmanDiagram rows={12} cols={12} sensorCells={[38, 42, 46, 86, 90, 94, 134, 138, 142]} />
        )}
        <p>計算していることは 2 段階で、どちらも行列とベクトルの積である。</p>
        <ol>
          <li>
            予測。前の周期の推定を <Tex math="A" /> で 1 ステップ進め、前の周期に加えた電力の効果 <Tex math="B u_{k-1}" /> を足す。外乱 <Tex math="d" /> はそのまま保つ。
          </li>
          <li>
            更新。予測から期待される測定 <Tex math={String.raw`\hat{y} = C \hat{x} + \hat{d}`} /> と実際の測定 <Tex math="y_k" /> の差にゲイン <Tex math="L" /> をかけて、推定に足す。
          </li>
        </ol>
        <p>
          ゲイン <Tex math="L" /> は「モデルと測定のどちらをどれだけ信じるか」を決める重みで、モデルの雑音の分散 <Tex math="Q" /> と測定雑音の分散 <Tex math="R" /> から一度だけ計算する。
          <Tex math="Q" /> を大きくすると測定寄りに、<Tex math="R" /> を大きくするとモデル寄りになる。
          このシミュレーションでは外乱の分散 <Tex math="Q_d" /> を大きくして、ずれを外乱として素早く取り込むようにしている。
        </p>
        <p>
          状態と外乱をまとめた <Tex math={String.raw`\xi = [x^\top \ d^\top]^\top`} /> について、式で書くと次のようになる。
        </p>
        <Tex
          block
          math={String.raw`\hat\xi_{k|k} = \hat\xi_{k|k-1} + L \left( y_k - C_\xi \hat\xi_{k|k-1} \right), \qquad Q = \operatorname{diag}(Q_x I, \, Q_d I), \ R`}
        />
        <p>
          ゲイン <Tex math="L" /> は離散リッカチ方程式から求める。
          <Tex math="Q_d" /> を <Tex math="Q_x" /> より大きくして、モデルとのずれを外乱として素早く取り込み、定常偏差を残さない。
        </p>

        <h3>予測と最適化</h3>
        <p>
          <Tex math="N_c" /> ステップより先は、最後の操作量を保持すると考える。
        </p>
        <Tex
          block
          math={String.raw`\hat{y}_{k+i} = C A^i \hat{x}_k + \sum_{j=0}^{i-1} C A^{i-1-j} B \, u_{k+\min(j, N_c - 1)} + \hat{d}_k, \quad i = 1, \dots, N_p`}
        />
        <p>予測を縦に並べると、操作量の列 <Tex math="U" /> についての一次式になる。</p>
        <Tex
          block
          math={String.raw`Y = \Phi \hat{x}_k + \Gamma U + \mathbf{1} \otimes \hat{d}_k, \quad \Phi \in \mathbb{R}^{N_p n_y \times n_x}, \ \Gamma \in \mathbb{R}^{N_p n_y \times N_c n_u}`}
        />
        <p>
          評価関数は、参照軌道との差と、操作量の変化の大きさの和とする。
          参照軌道は現在の値ではなく、予測ホライズンの各ステップの値 <Tex math="r_{k+i}" /> を使う。
        </p>
        <Tex
          block
          math={String.raw`J = \sum_{i=1}^{N_p} \left\| \hat{y}_{k+i} - r_{k+i} \right\|^2 + \lambda \sum_{j=0}^{N_c - 1} \left\| u_{k+j} - u_{k+j-1} \right\|^2, \quad 0 \le u_{k+j} \le u_\mathrm{max}`}
        />
        <p>上下限だけの制約なので、この問題は有界な最小二乗問題として解ける。</p>
        <WithResult result={result}>
          {(r) => {
            const m = r.controllers.MPC.info;
            return (
              <p>
                このシミュレーションでは <Tex math={`T_s = ${m.sampleTime}`} /> s、<Tex math={`N_p = ${m.horizon}`} /> ({(m.horizon * m.sampleTime).toFixed(0)} s)、
                <Tex math={`N_c = ${m.controlHorizon}`} /> ({(m.controlHorizon * m.sampleTime).toFixed(0)} s)、<Tex math={`\\lambda = ${m.moveWeight}`} />、
                <Tex math={`n_x = ${m.states}`} />、<Tex math={`n_u = n_y = ${r.rows * r.cols}`} />、<Tex math={`u_\\mathrm{max} = ${r.powerMax.toFixed(2)}`} /> W とした。
              </p>
            );
          }}
        </WithResult>

        <h3>各制御周期の演算</h3>
        <MpcCycleDiagram />
        <p>
          制御周期 <Tex math="T_s" /> ごとに、図の左から右へ次の手順を順に行う。
          {mpc && heaters !== null && (
            <>
              {" "}
              行列の大きさはこのシミュレーションの設定で、<Tex math={`n_x = ${mpc.states}`} />、<Tex math={`n_y = n_u = ${heaters}`} />、
              <Tex math={`N_p = ${mpc.horizon}`} />、<Tex math={`N_c = ${mpc.controlHorizon}`} /> である。
            </>
          )}
        </p>
        <ol>
          <li>
            測定。サーミスタの温度 <Tex math={String.raw`y_k \in \mathbb{R}^{n_y}`} /> を読む。
          </li>
          <li>
            状態推定の予測。前の周期の操作量で 1 ステップ進める。
            <Tex block math={String.raw`\hat\xi_{k|k-1} = A_\xi \hat\xi_{k-1} + B_\xi u_{k-1}, \qquad A_\xi \in \mathbb{R}^{(n_x + n_y) \times (n_x + n_y)}`} />
          </li>
          <li>
            状態推定の更新。測定との差にオフラインで求めたゲイン <Tex math={String.raw`L \in \mathbb{R}^{(n_x + n_y) \times n_y}`} /> をかけて補正し、
            <Tex math={String.raw`\hat{x}_k`} /> と <Tex math={String.raw`\hat{d}_k`} /> に分ける。
          </li>
          <li>
            目標ベクトルの構成。参照軌道の先読みから、外乱の推定値と自由応答を引く。
            <Tex block math={String.raw`b_k = \begin{bmatrix} r_{k+1} - \hat{d}_k \\ \vdots \\ r_{k+N_p} - \hat{d}_k \end{bmatrix} - \Phi \hat{x}_k \in \mathbb{R}^{N_p n_y}`} />
          </li>
          <li>
            二次計画問題を解く。操作量の列 <Tex math={String.raw`U \in \mathbb{R}^{N_c n_u}`} /> についての箱型制約付きの二次計画になる。
            <Tex
              block
              math={String.raw`\min_{U} \ \tfrac{1}{2} U^\top H U + g_k^\top U \quad \text{s.t.} \quad 0 \le U \le u_\mathrm{max}, \qquad H = \Gamma^\top \Gamma + \lambda D^\top D, \quad g_k = -\Gamma^\top b_k - \lambda D^\top U_{k-1}^\mathrm{hold}`}
            />
            <Tex math="D" /> は隣り合う操作量の差を取る行列、<Tex math={String.raw`U_{k-1}^\mathrm{hold}`} /> は前の周期の操作量を先頭に置いた列である。
            ヘッセ行列 <Tex math="H" /> は設定だけで決まるので、オフラインで一度計算すればよい。
          </li>
          <li>
            適用。解の先頭 <Tex math="n_u" /> 個を今の周期の電力 <Tex math="u_k" /> として出し、次の周期まで保持する。残りは捨てる。
          </li>
        </ol>
        <p>
          このシミュレーションでは、5 の二次計画を <Tex math="H" /> を作らずに、
          <Tex math={String.raw`\left\| \begin{bmatrix} \Gamma \\ \sqrt{\lambda} D \end{bmatrix} U - \begin{bmatrix} b_k \\ \sqrt{\lambda} U_{k-1}^\mathrm{hold} \end{bmatrix} \right\|^2`} />{" "}
          を最小にする上下限付き最小二乗問題として SciPy の <code>lsq_linear</code> に渡している。
          既定の解法は信頼領域反射法 (trust-region reflective) で、反復のたびに{" "}
          <Tex math={mpc && heaters !== null ? `${mpc.horizon * heaters + mpc.controlHorizon * heaters} \\times ${mpc.controlHorizon * heaters}` : String.raw`(N_p + N_c) n_u \times N_c n_u`} />{" "}
          の行列を扱う。前の周期の解からの温間開始 (warm start) はしていない。
        </p>
        <p>
          オフラインで済むのは、モデルの離散化、カルマンゲイン <Tex math="L" />、予測行列 <Tex math={String.raw`\Phi, \Gamma`} />、ヘッセ行列 <Tex math="H" /> である。
          オンラインで毎周期行うのは、2 から 4 の行列とベクトルの積と、5 の反復である。
        </p>
        <p>
          組み込みで実行するなら、上下限だけの二次計画は射影勾配法や ADMM で解ける。
          1 反復の中心は <Tex math="H U" /> の積和で、その回数は{" "}
          <Tex math={mpc && heaters !== null ? `(N_c n_u)^2 = ${(mpc.controlHorizon * heaters) ** 2}` : "(N_c n_u)^2"} /> である。
          前の周期の解を初期値にすれば、数十回の反復で収束する。
        </p>

        <h3>昇温レートの制御</h3>
        <p>
          昇温レートを与えると参照軌道が傾斜になり、MPC はその先の値を見ながら傾斜に沿って温度を上げる。
          目標に向かって一気に電力を上げるのではなく、傾斜を保つのに必要な電力だけを加える。
        </p>
        <p>PID 制御も同じ傾斜の参照軌道を追うが、先の値は見ないため、傾斜の開始と終了で遅れや行き過ぎが出る。</p>
        <p>
          温度の変化率そのものを不等式制約として課す方法もあるが、上下限以外の制約を扱う二次計画法の解法が必要になるため、ここでは参照軌道で表している。
        </p>

        <h3>デモ: MPC の設定を動かして PID と比べる</h3>
        <p>
          PID のデモと同じ 1 次元の模型で、MPC の Ts、Np、Nc、λ を動かす。予測モデルには実際の基板とずれた熱コンダクタンスも与えられ、
          カルマンフィルタの外乱の推定がそのずれをどう吸収するかも見える。
        </p>
        <MpcPlayground />

        <h3>制御周期の選び方</h3>
        <p>
          制御周期は、追従させたい最も速い応答の時定数の 1/10 から 1/20 が目安である。
          ヒーター直下のサーミスタの一次遅れの時定数が 1 分前後なら、3 から 6 s になる。
        </p>
        <p>
          長くすると二次計画を解く回数が減り、同じ <Tex math="N_p" /> でも秒数で見た予測区間が伸びる。
          一方で、外乱や隣からの入熱に気づいて電力を変えるまでに最大 <Tex math="T_s" /> の遅れが入り、傾斜の参照は階段状になる。
        </p>
        <p>
          計算量を減らしたいときは、制御周期を伸ばすより、<Tex math="N_c" /> を小さくするか、同じ電力を数ステップ保持する move blocking を使う方が追従を保ちやすい。
        </p>
        {pid && (
          <p className="muted small">
            今の基板の一次遅れの時定数は {pid.fopdt.tau.toFixed(0)} s、むだ時間は {pid.fopdt.delay.toFixed(0)} s である。
          </p>
        )}
      </section>

      <section className="card" id="notes-kalman">
        <h2>6. カルマンフィルタの計算例</h2>
        <p>
          5 節では役割を説明した。ここでは、同じ 1 次元の模型で、フィルタが毎周期に実際にどんな数値を作っているかを追う。
          ヒーターには決まった電力を入れ、制御はせずに推定だけを行う。
        </p>
        <h3>用意しておくもの</h3>
        <p>
          状態 <Tex math="x" /> (15 格子の温度上昇) と出力外乱 <Tex math="d" /> (3 個) をまとめた <Tex math={String.raw`\xi = [x^\top \ d^\top]^\top`} /> について、
          1 ステップ進める行列 <Tex math={String.raw`A_\xi`} />、電力の効果 <Tex math={String.raw`B_\xi`} />、測定を取り出す <Tex math={String.raw`C_\xi`} /> を作る。
        </p>
        <Tex
          block
          math={String.raw`A_\xi = \begin{bmatrix} A & 0 \\ 0 & I \end{bmatrix}, \quad B_\xi = \begin{bmatrix} B \\ 0 \end{bmatrix}, \quad C_\xi = \begin{bmatrix} C & I \end{bmatrix}`}
        />
        <p>
          ゲイン <Tex math="L" /> は、予測誤差の共分散 <Tex math="P" /> についての離散リッカチ方程式を、変化がなくなるまで繰り返して求める。
          これは設定が決まれば一度だけ行う計算で、制御中は <Tex math="L" /> を定数として使う。
        </p>
        <Tex
          block
          math={String.raw`P \leftarrow A_\xi \left( P - P C_\xi^\top (C_\xi P C_\xi^\top + R)^{-1} C_\xi P \right) A_\xi^\top + Q, \qquad L = P C_\xi^\top (C_\xi P C_\xi^\top + R)^{-1}`}
        />
        <p>
          <Tex math="Q" /> はモデルがどれだけ信用できないか、<Tex math="R" /> は測定がどれだけ信用できないかを表す分散である。
          <Tex math="Q" /> を大きくすると <Tex math="L" /> が大きくなって測定寄りになり、<Tex math="R" /> を大きくすると <Tex math="L" /> が小さくなってモデル寄りになる。
        </p>
        <h3>毎周期の計算</h3>
        <Tex
          block
          math={String.raw`\begin{aligned} \hat\xi_{k|k-1} &= A_\xi \hat\xi_{k-1} + B_\xi u_{k-1} & \text{(予測)} \\ \hat{y} &= C_\xi \hat\xi_{k|k-1} & \text{(期待される測定)} \\ \hat\xi_k &= \hat\xi_{k|k-1} + L \left( y_k - \hat{y} \right) & \text{(更新)} \end{aligned}`}
        />
        <p>
          3 つの測定の差は、<Tex math="L" /> の列を通じて 18 個全ての状態に配られる。測っていない格子の推定は、この配分と、隣の格子から <Tex math="A" /> を通じて伝わる情報だけで決まる。
        </p>
        <h3>デモ: 数値を追う</h3>
        <p>
          雑音やモデルのずれ、<Tex math="Q" /> と <Tex math="R" /> を動かし、時刻のスライダーで 1 周期の予測、測定、差、補正、更新の値を見る。
          「出力外乱 d の状態を持つ」を外すと、モデルのずれが定常偏差として残ることが分かる。
        </p>
        <KalmanPlayground />
      </section>
    </article>
  );
}
