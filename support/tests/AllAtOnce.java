// Example that uses many constructs to test the interpreter correctness
public class AllAtOnce {
    public static int main(String[] args) {
        int result = 0;

        int i = 0;

        for (i = 0;i < 10;i++) {
            result += add2IfGreaterThan4(i);
        }

        return result;
    }

    public static int add2IfGreaterThan4(int a) {
        if (a > 4) {
            return a + 2;
        }
        return a;
    }

    public static int sub2IfGreaterThan4(int a) {
        if (a > 4) {
            return a - 2;
        }
        return a;
    }

    public static int mul2IfGreaterThan4Div2IfLess(int a) {
        if (a > 4) {
            return a * 2;
        }
        return a / 2;
    }
}
