package org.bndroid.demo;

/**
 * Pure-static DEX entry points retained so the resource fixture can replace the
 * Activity-0 fixture in an end-to-end runtime without losing its tap probe.
 */
public final class Main {
    private Main() {}

    public static int boot() {
        return 20260729;
    }

    public static int onTap(int value) {
        return value + 7;
    }
}
