import java.util.ArrayList;

/** An owned application input for the unmodified upstream ART runtime. */
public final class Main {
    public static void main(String[] arguments) throws Exception {
        ArrayList<String> values = new ArrayList<>();
        values.add("Bndroid");
        values.add("ART");
        int sum = 0;
        for (int i = 0; i < 100; ++i) sum += i;
        if (sum != 4950 || !String.join("/", values).equals("Bndroid/ART")) {
            throw new AssertionError("Core Java result differs");
        }
        try {
            Integer.parseInt("invalid");
            throw new AssertionError("Missing Java exception");
        } catch (NumberFormatException expected) {
            System.out.println("ART_JAVA_CORE_OK sum=" + sum + " exceptions=1 collections=1");
        }
    }
}
