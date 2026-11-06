import java.lang.ref.ReferenceQueue;
import java.lang.ref.WeakReference;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

/** Real ART/GC/JNI workload; it does not implement any Android framework API. */
public final class ManagedMain {
    private static final Object monitor = new Object();
    private static final AtomicInteger callbacks = new AtomicInteger();
    private static volatile int retainedChecksum;
    private static int count;
    private static native int mutate(int[] values);
    private static native int attachedCallback();
    private static native String nativePath();
    private static native void reject();

    private static void check(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }
    private static int callback(int value) {
        callbacks.incrementAndGet();
        return value + 1;
    }
    private static WeakReference<byte[]> collectible(ReferenceQueue<byte[]> queue) {
        return new WeakReference<>(new byte[1024], queue);
    }
    private static void clocks() throws Exception {
        long wall = System.currentTimeMillis(), monotonic = System.nanoTime();
        check(wall > 1600000000000L, "RTC epoch");
        Thread.sleep(30);
        check(System.currentTimeMillis() - wall >= 20, "wall clock advances");
        check(System.nanoTime() - monotonic >= 20000000, "monotonic clock advances");
        System.out.println("ART_MANAGED_STAGE clocks");
    }
    private static void threads() throws Exception {
        CountDownLatch ready = new CountDownLatch(3), start = new CountDownLatch(1);
        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread[] workers = new Thread[3];
        for (int n = 0; n < workers.length; ++n) {
            workers[n] = new Thread(() -> {
                ready.countDown();
                try {
                    check(start.await(10, TimeUnit.SECONDS), "start deadline");
                    for (int i = 0; i < 500; ++i) {
                        synchronized (monitor) { ++count; }
                        if ((i & 31) == 0) Thread.yield();
                    }
                } catch (Throwable error) { failure.set(error); }
            }, "managed-worker-" + n);
            workers[n].start();
        }
        check(ready.await(10, TimeUnit.SECONDS), "workers started");
        start.countDown();
        for (Thread worker : workers) {
            worker.join(10000);
            check(!worker.isAlive(), "worker joined");
        }
        if (failure.get() != null) throw new AssertionError("worker failure", failure.get());
        check(count == 1500, "monitor serializes updates");
        CountDownLatch sleeping = new CountDownLatch(1);
        AtomicInteger interrupted = new AtomicInteger();
        Thread sleeper = new Thread(() -> {
            sleeping.countDown();
            try { Thread.sleep(10000); }
            catch (InterruptedException expected) { interrupted.incrementAndGet(); }
        }, "managed-interrupt");
        sleeper.start();
        check(sleeping.await(10, TimeUnit.SECONDS), "sleeper started");
        sleeper.interrupt(); sleeper.join(10000);
        check(!sleeper.isAlive() && interrupted.get() == 1, "Java interruption");
        System.out.println("ART_MANAGED_STAGE threads");
    }
    private static void garbageCollection() {
        ReferenceQueue<byte[]> queue = new ReferenceQueue<>();
        WeakReference<byte[]> reference = collectible(queue);
        byte[][][] survivors = new byte[16][][];
        for (int round = 0; round < 1024; ++round) {
            // Small arrays and reference graphs exercise region evacuation,
            // rather than allocating only non-moving large-object arrays.
            byte[][] graph = new byte[64][];
            for (int child = 0; child < graph.length; ++child) {
                byte[] bytes = new byte[1024];
                bytes[0] = (byte)round; bytes[bytes.length - 1] = (byte)child;
                graph[child] = bytes;
            }
            survivors[round % survivors.length] = graph;
            if ((round & 127) == 127) System.gc();
        }
        System.gc();
        for (int slot = 0; slot < survivors.length; ++slot) {
            byte[][] graph = survivors[slot];
            for (int child = 0; child < graph.length; ++child) {
                byte[] bytes = graph[child];
                check(bytes[0] == (byte)(1008 + slot) && bytes[bytes.length - 1] == (byte)child, "live graph survives copying GC");
                retainedChecksum += bytes[0] + bytes[bytes.length - 1];
            }
        }
        check(reference.get() == null, "GC clears unreachable object");
        check(queue.poll() == reference, "reference queue delivery");
        System.out.println("ART_MANAGED_STAGE gc");
    }
    private static void jni() {
        try {
            System.load("/system/lib64/libbndroidjni.so");
            throw new AssertionError("non-public system library escaped namespace");
        } catch (UnsatisfiedLinkError expected) {
            check(expected.getMessage().contains("not accessible"), "namespace denies existing non-public library");
        }
        System.loadLibrary("bndroidjni");
        check(System.getProperty("java.class.path").equals("/data/app/bndroid.runtime/base.apk"), "APK class path");
        check(nativePath().contains("base.apk!/lib/arm64-v8a/libbndroidjni.so"), "JNI loaded directly from APK");
        int[] values = {1, 2, 3};
        check(mutate(values) == 6 && values[0] == 11 && values[2] == 13, "JNI array roundtrip");
        check(attachedCallback() == 42 && callbacks.get() == 1, "attached pthread callback");
        try { reject(); throw new AssertionError("missing JNI exception"); }
        catch (IllegalArgumentException expected) {
            check("native rejection".equals(expected.getMessage()), "JNI exception text");
        }
        System.out.println("ART_MANAGED_STAGE jni");
        System.out.println("ART_APK_INPUT_OK dex=apk native=apk namespace=isolated");
    }
    public static void main(String[] arguments) throws Exception {
        clocks(); threads(); garbageCollection(); jni();
        System.out.println("ART_MANAGED_OK java_threads=3 monitors=1 interrupt=1 gc=1 jni=1 native_thread=1 clocks=1");
    }
}
