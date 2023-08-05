public class FunctionInsideTrace {
  public static void main(String[] args) {
    int j = 0;
    for (int i = 0; i < 10000; i++) {
      j = add(j, i);
    }
    System.out.println(j);
  }

  public static int add(int a, int b) {
    return a + b;
  }
}