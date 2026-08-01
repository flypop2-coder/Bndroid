package org.bndroid.demo;

/**
 * Pure-static DEX-0 entry points. These methods intentionally avoid framework,
 * allocation, exceptions, synchronization, native calls, and method calls.
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
