//! Crout LU with partial pivoting, ported from CircuitJS1
//! `SimulationManager.lu_factor_dense` / `lu_solve_dense`.

/// Factor `a` in place into LU. `ipvt` receives pivot indices.
/// Returns `false` if the matrix is singular.
pub fn lu_factor_dense(a: &mut [Vec<f64>], ipvt: &mut [i32]) -> bool {
    let n = a.len();
    if n == 0 {
        return true;
    }

    for i in 0..n {
        let mut row_all_zeros = true;
        for j in 0..n {
            if a[i][j] != 0.0 {
                row_all_zeros = false;
                break;
            }
        }
        if row_all_zeros {
            return false;
        }
    }

    for j in 0..n {
        for i in 0..j {
            let mut q = a[i][j];
            for k in 0..i {
                q -= a[i][k] * a[k][j];
            }
            a[i][j] = q;
        }

        let mut largest = 0.0;
        let mut largest_row: i32 = -1;
        for i in j..n {
            let mut q = a[i][j];
            for k in 0..j {
                q -= a[i][k] * a[k][j];
            }
            a[i][j] = q;
            let x = q.abs();
            if x >= largest {
                largest = x;
                largest_row = i as i32;
            }
        }

        if j as i32 != largest_row {
            if largest_row < 0 {
                return false;
            }
            let lr = largest_row as usize;
            for k in 0..n {
                let x = a[lr][k];
                a[lr][k] = a[j][k];
                a[j][k] = x;
            }
        }

        ipvt[j] = largest_row;

        if a[j][j] == 0.0 {
            return false;
        }

        if j != n - 1 {
            let mult = 1.0 / a[j][j];
            for i in (j + 1)..n {
                a[i][j] *= mult;
            }
        }
    }
    true
}

/// Solve `A x = b` using a factorization from [`lu_factor_dense`].
/// On return `b` contains `x`.
pub fn lu_solve_dense(a: &[Vec<f64>], ipvt: &[i32], b: &mut [f64]) {
    let n = a.len();
    if n == 0 {
        return;
    }

    let mut i = 0;
    while i != n {
        let row = ipvt[i] as usize;
        let swap = b[row];
        b[row] = b[i];
        b[i] = swap;
        if swap != 0.0 {
            break;
        }
        i += 1;
    }

    let bi = i;
    i += 1;
    while i < n {
        let row = ipvt[i] as usize;
        let mut tot = b[row];
        b[row] = b[i];
        for j in bi..i {
            tot -= a[i][j] * b[j];
        }
        b[i] = tot;
        i += 1;
    }

    i = n;
    while i > 0 {
        i -= 1;
        let mut tot = b[i];
        for j in (i + 1)..n {
            tot -= a[i][j] * b[j];
        }
        b[i] = tot / a[i][i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mat(rows: &[&[f64]]) -> Vec<Vec<f64>> {
        rows.iter().map(|r| r.to_vec()).collect()
    }

    #[test]
    fn solves_2x2() {
        let mut a = mat(&[&[2.0, 1.0], &[1.0, 2.0]]);
        let mut ipvt = [0i32; 2];
        assert!(lu_factor_dense(&mut a, &mut ipvt));
        let mut b = [3.0, 3.0];
        lu_solve_dense(&a, &ipvt, &mut b);
        assert!((b[0] - 1.0).abs() < 1e-12);
        assert!((b[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn solves_identity() {
        let mut a = mat(&[&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0]]);
        let mut ipvt = [0i32; 3];
        assert!(lu_factor_dense(&mut a, &mut ipvt));
        let mut b = [4.0, 5.0, 6.0];
        lu_solve_dense(&a, &ipvt, &mut b);
        assert_eq!(b, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn rejects_singular() {
        let mut a = mat(&[&[1.0, 2.0], &[2.0, 4.0]]);
        let mut ipvt = [0i32; 2];
        assert!(!lu_factor_dense(&mut a, &mut ipvt));
    }

    #[test]
    fn rejects_zero_row() {
        let mut a = mat(&[&[0.0, 0.0], &[1.0, 1.0]]);
        let mut ipvt = [0i32; 2];
        assert!(!lu_factor_dense(&mut a, &mut ipvt));
    }

    #[test]
    fn voltage_divider_mna() {
        // 10 V, 1k+1k to ground. Node 1 = source+, node 2 = mid.
        // VS extra unknown is row 2 (0-based).
        // A:
        //  [ 1/R1,      -1/R1,     1 ]
        //  [ -1/R1,  1/R1+1/R2,    0 ]
        //  [ 1,          0,        0 ]
        // B: [0, 0, 10]
        let g = 1.0 / 1000.0;
        // VS n1=gnd n2=node1: A[node1, vs] = -1, A[vs, node1] = 1, B[vs] = 10
        let mut a = mat(&[&[g, -g, -1.0], &[-g, 2.0 * g, 0.0], &[1.0, 0.0, 0.0]]);
        let mut ipvt = [0i32; 3];
        assert!(lu_factor_dense(&mut a, &mut ipvt));
        let mut b = [0.0, 0.0, 10.0];
        lu_solve_dense(&a, &ipvt, &mut b);
        assert!((b[0] - 10.0).abs() < 1e-9);
        assert!((b[1] - 5.0).abs() < 1e-9);
        assert!((b[2] - 0.005).abs() < 1e-12);
    }
}
