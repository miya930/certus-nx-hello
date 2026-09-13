// 行列は行優先で平坦に並べた配列で扱う。

pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(p, q)| p * q).sum()
}

/// out = M x。cols は M の列数。
pub fn mat_vec(m: &[f32], cols: usize, x: &[f32], out: &mut [f32]) {
    for (row, y) in m.chunks_exact(cols).zip(out.iter_mut()) {
        *y = dot(row, x);
    }
}

/// out = Mᵀ x。cols は M の列数。
pub fn mat_t_vec(m: &[f32], cols: usize, x: &[f32], out: &mut [f32]) {
    out.fill(0.0);
    for (row, xi) in m.chunks_exact(cols).zip(x) {
        for (o, v) in out.iter_mut().zip(row) {
            *o += v * xi;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: [f32; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

    #[test]
    fn mat_vec_multiplies_rows() {
        let mut out = [0.0; 2];
        mat_vec(&M, 3, &[1.0, 0.0, -1.0], &mut out);
        assert_eq!(out, [-2.0, -2.0]);
    }

    #[test]
    fn mat_t_vec_multiplies_columns() {
        let mut out = [0.0; 3];
        mat_t_vec(&M, 3, &[1.0, -1.0], &mut out);
        assert_eq!(out, [-3.0, -3.0, -3.0]);
    }
}
