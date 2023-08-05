public class MixedArg {
  public static void main(String[] args) {
    int i = 1;
    long l = 2;
    float f = 3;
    double d = 4;
    printSum(i, l, f, d);
  }

  public static void printSum(int i, long l, float f, double d) {
    double res = i + l + f + d;
    System.out.println(res);
  }
}