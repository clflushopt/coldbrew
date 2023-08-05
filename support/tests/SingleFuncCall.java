public class SingleFuncCall {
  public static void main(String[] args) {
    int res = add(3, 2);
    System.out.println(res);
  }

  static int add(int a, int b) {
    return a + b;
  }
}
