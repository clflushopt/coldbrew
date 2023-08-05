public class FloatFibonacci {
  public static void main(String[] args) {
    System.out.println(fibonacci(20f));
  }
  public static float fibonacci(float n) {
    if (n <= 2f)
      return 1f;
    else
      return fibonacci(n - 1) + fibonacci(n - 2);
  }
}