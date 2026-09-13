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
          Optimization result <span className="muted">{result.seconds.toFixed(0)} s</span>
        </h2>
        <button type="button" className="ghost-button" onClick={onClose}>
          Close
        </button>
      </header>
      <p className="muted">
        見つかった値は Setup に書き戻した。Run simulation を押すと、この値で PID と MPC を実行する。誤差は参照軌道に対する平均絶対誤差である。
      </p>
      <div className="optimization-columns">
        {pid && (
          <div>
            <h3>PID: Kp, Ti, Td by Nelder–Mead</h3>
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
                  <th>Error on board [K]</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>Start (IMC, λ = 0.5τ)</td>
                  <td>{pid.start.kp.toFixed(4)}</td>
                  <td>{pid.start.ti.toFixed(1)}</td>
                  <td>{pid.start.td.toFixed(2)}</td>
                  <td>{pid.start.plantError.toFixed(3)}</td>
                </tr>
                <tr>
                  <td>Optimized</td>
                  <td className="diagonal">{pid.kp.toFixed(4)}</td>
                  <td className="diagonal">{pid.ti.toFixed(1)}</td>
                  <td className="diagonal">{pid.td.toFixed(2)}</td>
                  <td className="diagonal">{pid.plantError.toFixed(3)}</td>
                </tr>
              </tbody>
            </table>
            <p className="muted small">
              {pid.evaluations} evaluations on a 2 mm model, search error {pid.searchError.toFixed(3)} K, then verified on the 1 mm board.
            </p>
          </div>
        )}
        {mpc && (
          <div>
            <h3>MPC: move weight λ, then control horizon Nc</h3>
            <table className="data-table">
              <thead>
                <tr>
                  <th>
                    <Tex math="\lambda" />
                  </th>
                  <th>
                    <Tex math="N_c" />
                  </th>
                  <th>Error on board [K]</th>
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
              Start λ = {mpc.start.moveWeight}, Nc = {mpc.start.controlHorizon}: {mpc.start.plantError.toFixed(3)} K. Best: λ = {mpc.moveWeight}, Nc = {mpc.controlHorizon}:{" "}
              {mpc.plantError.toFixed(3)} K.
            </p>
          </div>
        )}
      </div>
    </section>
  );
}
