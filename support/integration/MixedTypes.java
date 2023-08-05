public class MixedTypes {
  public static void main(String[] args) {
    double d = mixedMul(2, 2L, 2.0f, 2.0);
    System.out.println(d);
  }

  public static double mixedMul(int i, long l, float f, double d) {
    return i * l * f * d;
  }
}