public class ChineseRemainder {
  public static long mul_inv(int a, int b) {
    int b0 = b, t, q;
    int x0 = 0, x1 = 1;
    if (b == 1)
      return 1;
    while (a > 1) {
      q = a / b;
      t = b;
      b = a % b;
      a = t;
      t = x0;
      x0 = x1 - q * x0;
      x1 = t;
    }
    if (x1 < 0)
      x1 += b0;
    return x1;
  }
  public static int chinese_remainder(int n1, int n2, int n3, int a1, int a2, int a3) {
    int p, i, prod = 1, sum = 0;
    prod = n1 * n2 * n3;
    p = prod / n1;
    sum += a1 * mul_inv(p, n1) * p;
    p = prod / n2;
    sum += a2 * mul_inv(p, n2) * p;
    p = prod / n3;
    sum += a3 * mul_inv(p, n3) * p;
    return ((sum % prod) + prod) % prod;
  }

  public static void main(String[] args) {
    System.out.println(chinese_remainder(997, 991, 983, 123, 14, 66));
  }
}
