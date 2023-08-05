public class MultiFuncCall {
    public static void main(String[] args) {
        int sum = 0;
        int a = 4;
        int b = 3;
        int c = 2;
        for (int i = 0;i < 10;i++) {
            sum += threeArgs(a,b,c);
        }
        System.out.println(sum);
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public static int sub(int a, int b) {
        return a - b;
    }

    public static int threeArgs(int a, int b, int c) {
        return add(a, sub(b,c));
    }
}
