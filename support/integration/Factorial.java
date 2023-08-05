public class Factorial {
  public static void main(String[] args) {
    int f = factorial(12);
    System.out.println(f);
  }

  public static int factorial(int n) {
    int accumulator = 1;
    for (int i = 2; i <= n; i++) accumulator *= i;
    return accumulator;
  }
}