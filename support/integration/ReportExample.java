public class ReportExample {
  public static void main(String[] args) {
    foo();
    System.out.println(-1);
  }
  public static void foo() {
    int i = 0;
    for (int j = 0; j < 100000; j++) {
      if (j > 66666) {
        i += 1;
      } else if (j > 33333) {
        i += 2;
      } else {
        i += 3;
      }
    }
  }
}