/**
 * Crout LU copied from SimulationManager.lu_factor_dense / lu_solve_dense
 * so LU test cases can be checked against the Java implementation.
 */
public class CroutOracle {
    static boolean lu_factor_dense(double a[][], int n, int ipvt[]) {
	int i,j,k;
	for (i = 0; i != n; i++) {
	    boolean row_all_zeros = true;
	    for (j = 0; j != n; j++) {
		if (a[i][j] != 0) {
		    row_all_zeros = false;
		    break;
		}
	    }
	    if (row_all_zeros)
		return false;
	}
	for (j = 0; j != n; j++) {
	    for (i = 0; i != j; i++) {
		double q = a[i][j];
		for (k = 0; k != i; k++)
		    q -= a[i][k]*a[k][j];
		a[i][j] = q;
	    }
	    double largest = 0;
	    int largestRow = -1;
	    for (i = j; i != n; i++) {
		double q = a[i][j];
		for (k = 0; k != j; k++)
		    q -= a[i][k]*a[k][j];
		a[i][j] = q;
		double x = Math.abs(q);
		if (x >= largest) {
		    largest = x;
		    largestRow = i;
		}
	    }
	    if (j != largestRow) {
		if (largestRow == -1)
		    return false;
		double x;
		for (k = 0; k != n; k++) {
		    x = a[largestRow][k];
		    a[largestRow][k] = a[j][k];
		    a[j][k] = x;
		}
	    }
	    ipvt[j] = largestRow;
	    if (a[j][j] == 0.0)
		return false;
	    if (j != n-1) {
		double mult = 1.0/a[j][j];
		for (i = j+1; i != n; i++)
		    a[i][j] *= mult;
	    }
	}
	return true;
    }

    static void lu_solve_dense(double a[][], int n, int ipvt[], double b[]) {
	int i;
	for (i = 0; i != n; i++) {
	    int row = ipvt[i];
	    double swap = b[row];
	    b[row] = b[i];
	    b[i] = swap;
	    if (swap != 0)
		break;
	}
	int bi = i++;
	for (; i < n; i++) {
	    int row = ipvt[i];
	    int j;
	    double tot = b[row];
	    b[row] = b[i];
	    for (j = bi; j < i; j++)
		tot -= a[i][j]*b[j];
	    b[i] = tot;
	}
	for (i = n-1; i >= 0; i--) {
	    double tot = b[i];
	    int j;
	    for (j = i+1; j != n; j++)
		tot -= a[i][j]*b[j];
	    b[i] = tot/a[i][i];
	}
    }

    static void printVec(String name, boolean ok, double[] b) {
	System.out.print("{\"id\":\"" + name + "\",\"ok\":" + ok + ",\"x\":[");
	if (ok)
	    for (int i = 0; i < b.length; i++) {
		if (i > 0) System.out.print(",");
		System.out.print(b[i]);
	    }
	System.out.println("]}");
    }

    public static void main(String[] args) {
	{
	    double[][] a = {{2,1},{1,2}};
	    double[] b = {3,3};
	    int[] ipvt = new int[2];
	    boolean ok = lu_factor_dense(a, 2, ipvt);
	    if (ok) lu_solve_dense(a, 2, ipvt, b);
	    printVec("lu_solves_2x2", ok, b);
	}
	{
	    double[][] a = {{1,0,0},{0,1,0},{0,0,1}};
	    double[] b = {4,5,6};
	    int[] ipvt = new int[3];
	    boolean ok = lu_factor_dense(a, 3, ipvt);
	    if (ok) lu_solve_dense(a, 3, ipvt, b);
	    printVec("lu_solves_identity", ok, b);
	}
	{
	    double g = 1.0/1000.0;
	    double[][] a = {{g,-g,-1},{-g,2*g,0},{1,0,0}};
	    double[] b = {0,0,10};
	    int[] ipvt = new int[3];
	    boolean ok = lu_factor_dense(a, 3, ipvt);
	    if (ok) lu_solve_dense(a, 3, ipvt, b);
	    printVec("lu_voltage_divider_mna", ok, b);
	}
	{
	    double[][] a = {{1,2},{2,4}};
	    int[] ipvt = new int[2];
	    boolean ok = lu_factor_dense(a, 2, ipvt);
	    printVec("lu_rejects_singular", ok, new double[]{ok?1:0});
	}
	{
	    double[][] a = {{0,0},{1,1}};
	    int[] ipvt = new int[2];
	    boolean ok = lu_factor_dense(a, 2, ipvt);
	    printVec("lu_rejects_zero_row", ok, new double[]{ok?1:0});
	}
    }
}
