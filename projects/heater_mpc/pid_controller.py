"""各ヒーターを自分のサーミスタだけで制御する、ヒーターごとに独立した PID 制御。"""

import numpy as np

# 微分の高周波の利得を、比例の何倍までに抑えるか。
DERIVATIVE_FILTER = 10.0


class PidController:
    def __init__(self, kp, ti, td, dt, power_max, heaters):
        self.kp = kp
        self.ti = ti
        self.td = td
        self.dt = dt
        self.power_max = power_max
        self.integral = np.zeros(heaters)
        self.derivative = np.zeros(heaters)
        self.previous = None

    def update(self, setpoint, measurement):
        error = setpoint - measurement
        # 微分は測定値にかけ、目標値を変えたときに出力が跳ねないようにする。
        if self.previous is not None:
            tf = self.td / DERIVATIVE_FILTER
            slope = (measurement - self.previous) / self.dt
            self.derivative += (self.dt / (tf + self.dt)) * (-self.td * slope - self.derivative)
        self.previous = measurement

        unclamped = self.kp * (error + self.integral + self.derivative)
        power = np.clip(unclamped, 0.0, self.power_max)
        # 出力が飽和している向きには積分を進めず、ワインドアップを防ぐ。
        saturated = (unclamped != power) & (np.sign(error) == np.sign(unclamped - power))
        self.integral += np.where(saturated, 0.0, error * self.dt / self.ti)
        return power
