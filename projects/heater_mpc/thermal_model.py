"""格子に並べたヒーターとサーミスタを載せた基板の、2 次元の熱モデル。"""

from dataclasses import dataclass

import numpy as np
from scipy.sparse import diags, identity, kron
from scipy.sparse.linalg import factorized

# 材料の物性値。
K_FR4 = 0.3  # W/(m K)
RHO_FR4 = 1850.0  # kg/m^3
CP_FR4 = 1100.0  # J/(kg K)
K_CU = 385.0  # W/(m K)
RHO_CU = 8960.0  # kg/m^3
CP_CU = 385.0  # J/(kg K)

T_CU = 35e-6  # m、1 oz
COPPER_LAYERS = 2

# 自然対流と放射をまとめた、片面あたりの熱伝達率。
H_SIDE = 10.0  # W/(m^2 K)


@dataclass(frozen=True)
class Geometry:
    """基板とヒーターの寸法。長さの単位は m。"""

    rows: int
    cols: int
    pitch: float  # ヒーターの間隔
    margin: float  # 外側のヒーターの中心から基板の端までの距離
    heater_x: float  # ヒーターの外形
    heater_y: float
    sensor_offset: float  # ヒーターの中心からサーミスタまでの距離、y 方向
    thickness: float
    copper: float  # 各層で銅箔が占める割合

    @property
    def heaters(self):
        return self.rows * self.cols

    @property
    def width(self):
        return 2 * self.margin + (self.cols - 1) * self.pitch

    @property
    def height(self):
        return 2 * self.margin + (self.rows - 1) * self.pitch


# ヒーターは 2512 サイズのチップ抵抗を想定し、サーミスタはその長辺の横に置く。
DEFAULT_GEOMETRY = Geometry(
    rows=3, cols=3, pitch=20e-3, margin=10e-3, heater_x=6e-3, heater_y=3e-3,
    sensor_offset=3e-3, thickness=1.6e-3, copper=0.5,
)


def _second_difference(n):
    # 基板の端は断熱とし、端の格子は隣が 1 つ少ない。
    d = diags([-1.0, 2.0, -1.0], [-1, 0, 1], shape=(n, n)).tolil()
    d[0, 0] = 1.0
    d[-1, -1] = 1.0
    return d


class Board:
    """温度は周囲温度からの上昇 [K] で扱い、ヒーターの電力 [W] を入力とする。"""

    def __init__(self, geometry, dx):
        self.geometry = geometry
        self.dx = dx
        self.ny = self._cells(geometry.height)
        self.nx = self._cells(geometry.width)
        self.cells = self.ny * self.nx
        # 格子の番号は行優先で、row * nx + col とする。
        self.heater_cells = [
            (self._cells(geometry.margin + r * geometry.pitch), self._cells(geometry.margin + c * geometry.pitch))
            for r in range(geometry.rows)
            for c in range(geometry.cols)
        ]
        offset = self._cells(geometry.sensor_offset)
        self.sensor_cells = [(min(self.ny - 1, r + offset), c) for r, c in self.heater_cells]
        self.sensors = [r * self.nx + c for r, c in self.sensor_cells]

        # 銅箔を除いた厚さを FR4 とみなす。
        t_cu = T_CU * COPPER_LAYERS * geometry.copper
        t_fr4 = geometry.thickness - T_CU * COPPER_LAYERS
        self.conductance = K_FR4 * t_fr4 + K_CU * t_cu
        self.areal_capacity = RHO_FR4 * CP_FR4 * t_fr4 + RHO_CU * CP_CU * t_cu
        cell_area = dx * dx
        self.capacity = self.areal_capacity * cell_area
        self.length = np.sqrt(self.conductance / (2.0 * H_SIDE))

        conduction = self.conductance * (
            kron(_second_difference(self.ny), identity(self.nx)) + kron(identity(self.ny), _second_difference(self.nx))
        )
        self.stiffness = (conduction + 2.0 * H_SIDE * cell_area * identity(self.cells)).tocsc()
        self.heat_input = self._heat_input()

    def _cells(self, length):
        return int(round(length / self.dx))

    def _heat_input(self):
        """列 j が、j 番目のヒーターに 1 W を与えたときの熱入力になる行列を返す。"""
        half_x = self._cells(self.geometry.heater_x) // 2
        half_y = self._cells(self.geometry.heater_y) // 2
        q = np.zeros((self.cells, self.geometry.heaters))
        for j, (row, col) in enumerate(self.heater_cells):
            area = np.zeros((self.ny, self.nx))
            area[max(0, row - half_y) : row + half_y + 1, max(0, col - half_x) : col + half_x + 1] = 1.0
            q[:, j] = (area / area.sum()).ravel()
        return q

    def steady_gain(self):
        """gain[i, j] は、ヒーター j の 1 W あたりのセンサー i の温度上昇 [K/W]。"""
        solve = factorized(self.stiffness)
        columns = [solve(self.heat_input[:, j]) for j in range(self.geometry.heaters)]
        return np.column_stack(columns)[self.sensors]

    def stepper(self, dt):
        """後退オイラー法で dt だけ進める関数を返す。"""
        storage = self.capacity / dt
        solve = factorized((self.stiffness + storage * identity(self.cells)).tocsc())

        def step(temperature, power):
            return solve(self.heat_input @ power + storage * temperature)

        return step

    def discrete_model(self, dt):
        """x[k+1] = a x[k] + b u[k]、y[k] = c x[k] となる、密行列の離散時間モデルを返す。"""
        storage = self.capacity / dt
        system = self.stiffness.toarray() + storage * np.eye(self.cells)
        a = np.linalg.solve(system, storage * np.eye(self.cells))
        b = np.linalg.solve(system, self.heat_input)
        c = np.zeros((self.geometry.heaters, self.cells))
        c[np.arange(self.geometry.heaters), self.sensors] = 1.0
        return a, b, c
