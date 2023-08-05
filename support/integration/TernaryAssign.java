public class TernaryAssign {
  public static void main(String[] args) {
    int i = 0;
    for (int j = 0; j < 10; j++) {
      i += j < 5 ? 1 : -1;
    }
    System.out.println(i);
  }
}