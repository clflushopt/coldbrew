public class IsPrime {
  public static void main(String[] args) {
    int potentialPrime = 104729;
    int largestCheck = integerRoot(potentialPrime);
    for (int i = 2; i < largestCheck; i++) {
      if (potentialPrime % i == 0) {
        System.out.println(0);
        return;
      }
    }
    System.out.println(1);
  }

  public static int integerRoot(int num) {
    for (int i = 1; i < num; i++) {
      if (num % i == i) {
        return i;
      }
    }
    return num;
  }
}