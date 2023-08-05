public class DoubleFibonacci {
  public static void main(String[] args) {
    System.out.println(fibonacci(20.0));
  }
  public static double fibonacci(double n) {
    if (n <= 2.0)
      return 1.0;
    else
      return fibonacci(n - 1) + fibonacci(n - 2);
  }
}