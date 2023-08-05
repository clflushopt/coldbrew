public class MixedExecutionTwoVars {
  public static void main(String[] args) {
    int i = 0;
    int j = 7;
    i += j * 3;
    j *= 6;
    j -= 43;
    i *= 2;
    System.out.println(i + j);
  }
}