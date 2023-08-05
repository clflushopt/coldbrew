public class WhileLoopAtStart {
  public static void main(String[] args) {
    int j = 10;
    loopdiloop(j);
    System.out.println(j);
  }
  public static void loopdiloop(int j) {
    while (j < 15) {
      j++;
    }
  }
}