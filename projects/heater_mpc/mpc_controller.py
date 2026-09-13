"""熱モデルで全センサーの将来の温度を予測し、全ヒーターの電力をまとめて決めるモデル予測制御。"""

import numpy as np
from scipy.linalg import solve_discrete_are
from scipy.optimize import lsq_linear

# 状態推定に使う雑音の分散。
# モデルの誤差を出力の外乱が吸収するよう、外乱の分散を状態より大きくする。
STATE_NOISE = 1e-4
DISTURBANCE_NOISE = 1e-2
MEASUREMENT_NOISE = 1e-2


class MpcController:
    def __init__(self, a, b, c, power_max, horizon, control_horizon, move_weight):
        self.a = a
        self.b = b
        self.c = c
        self.power_max = power_max
        self.horizon = horizon
        self.control_horizon = control_horizon
        self.move_weight = move_weight
        # 制御周期ごとに、この数だけ先までの目標温度を受け取る。
        self.lookahead = horizon
        states, heaters = b.shape
        sensors = c.shape[0]

        # 出力に積分する外乱を足したモデルで状態を推定し、定常偏差を残さない。
        self.a_aug = np.block([[a, np.zeros((states, sensors))], [np.zeros((sensors, states)), np.eye(sensors)]])
        self.b_aug = np.vstack([b, np.zeros((sensors, heaters))])
        self.c_aug = np.hstack([c, np.eye(sensors)])
        noise = np.diag(np.r_[np.full(states, STATE_NOISE), np.full(sensors, DISTURBANCE_NOISE)])
        p = solve_discrete_are(self.a_aug.T, self.c_aug.T, noise, MEASUREMENT_NOISE * np.eye(sensors))
        s = self.c_aug @ p @ self.c_aug.T + MEASUREMENT_NOISE * np.eye(sensors)
        self.gain = np.linalg.solve(s, self.c_aug @ p).T

        self.free = self._free_response()
        self.forced = self._forced_response()
        self.estimate = np.zeros(states + sensors)
        self.power = np.zeros(heaters)

    def _free_response(self):
        """行 i が、現在の状態から i + 1 ステップ後のセンサーの温度を与える行列を返す。"""
        rows = []
        propagate = self.c
        for _ in range(self.horizon):
            propagate = propagate @ self.a
            rows.append(propagate)
        return np.vstack(rows)

    def _forced_response(self):
        """列が 1 つの操作量に、行が予測ホライズン内のセンサーの温度に対応する行列を返す。"""
        states, heaters = self.b.shape
        columns = []
        for move in range(self.control_horizon):
            for heater in range(heaters):
                x = np.zeros(states)
                outputs = []
                for step in range(self.horizon):
                    # 最後の操作量は、予測ホライズンの終わりまで保持する。
                    held = step == move or (move == self.control_horizon - 1 and step > move)
                    x = self.a @ x + (self.b[:, heater] if held else 0.0)
                    outputs.append(self.c @ x)
                columns.append(np.concatenate(outputs))
        return np.column_stack(columns)

    def update(self, reference, measurement):
        """reference は、行が予測ホライズンの各ステップ、列がセンサーの目標温度。"""
        predicted = self.a_aug @ self.estimate + self.b_aug @ self.power
        self.estimate = predicted + self.gain @ (measurement - self.c_aug @ predicted)
        states = self.a.shape[0]
        disturbance = self.estimate[states:]

        heaters = self.power.size
        target = (reference - disturbance).ravel() - self.free @ self.estimate[:states]

        # 操作量の変化にも重みを付け、急な電力の変化を抑える。
        moves = self.control_horizon * heaters
        difference = np.eye(moves) - np.eye(moves, k=-heaters)
        previous = np.zeros(moves)
        previous[:heaters] = self.power
        weight = np.sqrt(self.move_weight)
        solution = lsq_linear(
            np.vstack([self.forced, weight * difference]),
            np.concatenate([target, weight * previous]),
            bounds=(0.0, self.power_max),
        )
        self.power = solution.x[:heaters]
        return self.power
