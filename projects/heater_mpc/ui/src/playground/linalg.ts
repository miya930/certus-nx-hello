/** Notes のデモで使う、小さな密行列の演算。 */

export type Matrix = number[][];

export function zeros(rows: number, cols: number): Matrix {
  return Array.from({ length: rows }, () => new Array<number>(cols).fill(0));
}

export function identity(n: number): Matrix {
  const m = zeros(n, n);
  for (let i = 0; i < n; i++) m[i][i] = 1;
  return m;
}

export function transpose(a: Matrix): Matrix {
  return a[0].map((_, j) => a.map((row) => row[j]));
}

export function matmul(a: Matrix, b: Matrix): Matrix {
  const out = zeros(a.length, b[0].length);
  for (let i = 0; i < a.length; i++) {
    for (let k = 0; k < b.length; k++) {
      const aik = a[i][k];
      if (aik === 0) continue;
      for (let j = 0; j < b[0].length; j++) out[i][j] += aik * b[k][j];
    }
  }
  return out;
}

export function matvec(a: Matrix, v: number[]): number[] {
  return a.map((row) => row.reduce((s, x, j) => s + x * v[j], 0));
}

export function add(a: Matrix, b: Matrix): Matrix {
  return a.map((row, i) => row.map((x, j) => x + b[i][j]));
}

export function sub(a: Matrix, b: Matrix): Matrix {
  return a.map((row, i) => row.map((x, j) => x - b[i][j]));
}

export function scale(a: Matrix, s: number): Matrix {
  return a.map((row) => row.map((x) => x * s));
}

/** ガウスの消去法で逆行列を求める。対称正定値の小さな行列を想定する。 */
export function inverse(a: Matrix): Matrix {
  const n = a.length;
  const m = a.map((row, i) => [...row, ...identity(n)[i]]);
  for (let col = 0; col < n; col++) {
    let pivot = col;
    for (let r = col + 1; r < n; r++) {
      if (Math.abs(m[r][col]) > Math.abs(m[pivot][col])) pivot = r;
    }
    [m[col], m[pivot]] = [m[pivot], m[col]];
    const p = m[col][col];
    for (let j = 0; j < 2 * n; j++) m[col][j] /= p;
    for (let r = 0; r < n; r++) {
      if (r === col) continue;
      const f = m[r][col];
      if (f === 0) continue;
      for (let j = 0; j < 2 * n; j++) m[r][j] -= f * m[col][j];
    }
  }
  return m.map((row) => row.slice(n));
}

/** べき乗法で最大固有値を求める。QP の勾配法のステップ幅に使う。 */
export function largestEigenvalue(a: Matrix, iterations = 60): number {
  let v = a.map(() => 1);
  let value = 0;
  for (let i = 0; i < iterations; i++) {
    const w = matvec(a, v);
    const norm = Math.hypot(...w);
    if (norm === 0) return 0;
    value = norm;
    v = w.map((x) => x / norm);
  }
  return value;
}

export function maxAbsDiff(a: Matrix, b: Matrix): number {
  let d = 0;
  a.forEach((row, i) => row.forEach((x, j) => (d = Math.max(d, Math.abs(x - b[i][j])))));
  return d;
}
