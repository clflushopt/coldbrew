public class Loop10 {
  public static void main(String[] args) {
      int sum = 0;
      int i =1;
      for (i=1;i <= 10;i++) {
          int a = i * 2;
          int b = i * 3;
          int c = i * 4;
          sum = sum + (a * b) - c;
      }
      System.out.println(sum);
  }
}
