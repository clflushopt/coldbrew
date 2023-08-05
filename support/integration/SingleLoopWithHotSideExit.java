public class SingleLoopWithHotSideExit {
  public static void main(String[] args) {
    int i = 0;
    for (int j = 0; j < 10000000; j++) {
      if (j % 100 == 0) {
        i += 2;
      } else {
        i += 1;
      }
    }
    System.out.println(i);
  }
}