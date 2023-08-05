public class MixedExecution {
  public static void main(String[] args) {
    int i = 0;
    // Trace starts here
    i += 6;
    i += 7;
    i *= 3;
    i -= 9;
    i++;
    // Trace stops here
    System.out.println(i);
  }
}