import type { OptimizationResult } from "../api";
import { Tex } from "./Tex";

interface Props {
  result: OptimizationResult;
  onClose: () => void;
}

export function OptimizationCard({ result, onClose }: Props) {
  const { pid, mpc } = result;
  return (
    <section className="card optimization">
      <header className="card-header">
        <h2>
          最適化の結果 <span className="muted">{result.seconds.toFixed(0)} s</span>
        </h2>
        <button type="button" className="ghost-button" onClick={onClose}>
          閉じる
        </button>
      </header>
      <p className="muted">
        見つかった値は設定に書き戻した。「シミュレーションを実行」を押すと、この値で PID と MPC を実行する。誤差は参照軌道に対する平均絶対誤差である。
      </p>
      <div className="optimization-columns">
        {pid && (
          <div>
            <h3>PID: Nelder–Mead 法で Kp、Ti、Td を探索</h3>
            <table className="data-table">
              <thead>
                <tr>
                  <th />
                  <th>
                    <Tex math="K_p" /> [W/K]
                  </th>
                  <th>
                    <Tex math="T_i" /> [s]
                  </th>
                  <th>
                    <Tex math="T_d" /> [s]
                  </th>
                  <th>基板での誤差 [K]</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>開始 (IMC、λ = 0.5τ)</td>
                  <td>{pid.start.kp.toFixed(4)}</td>
                  <td>{pid.start.ti.toFixed(1)}</td>
                  <td>{pid.start.td.toFixed(2)}</td>
                  <td>{pid.start.plantError.toFixed(3)}</td>
                </tr>
                <tr>
                  <td>最適化後</td>
                  <td className="diagonal">{pid.kp.toFixed(4)}</td>
                  <td className="diagonal">{pid.ti.toFixed(1)}</td>
                  <td className="diagonal">{pid.td.toFixed(2)}</td>
                  <td className="diagonal">{pid.plantError.toFixed(3)}</td>
                </tr>
              </tbody>
            </table>
            <p className="muted small">
              2 mm 格子のモデルで {pid.evaluations} 回評価し (探索中の誤差 {pid.searchError.toFixed(3)} K)、1 mm の基板で確かめた。
            </p>
          </div>
        )}
        {mpc && (
          <div>
            <h3>MPC: λ、次に Nc を候補から選択</h3>
            <table className="data-table">
              <thead>
                <tr>
                  <th>
                    <Tex math="\lambda" />
                  </th>
                  <th>
                    <Tex math="N_c" />
                  </th>
                  <th>基板での誤差 [K]</th>
                </tr>
              </thead>
              <tbody>
                {mpc.trials.map((t, i) => {
                  const best = t.moveWeight === mpc.moveWeight && t.controlHorizon === mpc.controlHorizon && t.error === mpc.plantError;
                  return (
                    <tr key={i}>
                      <td className={best ? "diagonal" : ""}>{t.moveWeight}</td>
                      <td className={best ? "diagonal" : ""}>{t.controlHorizon}</td>
                      <td className={best ? "diagonal" : ""}>{t.error.toFixed(3)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            <p className="muted small">
              開始 λ = {mpc.start.moveWeight}、Nc = {mpc.start.controlHorizon}: {mpc.start.plantError.toFixed(3)} K。最良 λ = {mpc.moveWeight}、Nc = {mpc.controlHorizon}:{" "}
              {mpc.plantError.toFixed(3)} K。
            </p>
          </div>
        )}
      </div>
    </section>
  );
}
