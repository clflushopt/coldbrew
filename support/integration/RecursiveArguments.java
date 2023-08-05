public class RecursiveArguments {
  public static void main(String[] args) {
    int i = 10;
    int j = 5;
    int k = sub(i, sub(i, j));
    System.out.println(k);
  }

  public static int sub(int i, int j) {
    return i - j;
  }
}