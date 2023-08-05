public class TwoHotSideExits {
  public static void main(String[] args) {
    int i = 0;
    for (int j = 0; j < 300000; j++) {
      if (j < 100000) {
        i += 1;
      } else if (j < 200000) {
        i += 2;
      } else {
        i += 3;
      }
    }
    System.out.println(i);
  }
}