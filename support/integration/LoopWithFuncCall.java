
public class LoopWithFuncCall {
  public static void main(String[] args) {
    int i = 0, j = 0;
    for (int k = 0; k < 1000; k++) {
      i = threeArgs(i, j, k);
      j = threeArgs(j, k, i);
    }
    System.out.println(i);
    System.out.println(j);
  }

  public static int threeArgs(int a, int b, int c) {
    return a + b - c;
  }
}