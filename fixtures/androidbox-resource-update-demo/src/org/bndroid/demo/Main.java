package org.bndroid.demo;

/**
 * Pure-static DEX entry points retained across the update so Update-0 changes
 * only the package version and resource-backed Activity presentation.
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
