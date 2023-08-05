public class SingleLoopWithIfStatement {
  public static void main(String[] args) {
    int j = 0;
    for (int i = 0; i < 100000; i++) {
      if (j == 50000) {
        j -= 50000;
      } else {
        j++;
      }
    }
    System.out.println(j);
  }
}