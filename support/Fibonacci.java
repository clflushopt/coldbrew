public class Fibonacci {
  public static void main(String[] args) {
    int n = fibonacci(15);
    System.out.println(n);
  }
  public static int fibonacci(int n) {
    if (n <= 1)
      return 1;
    else
      return fibonacci(n - 1) + fibonacci(n - 2);
  }
}