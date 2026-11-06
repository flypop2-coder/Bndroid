import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Parcel;
import android.os.SystemClock;
import android.util.Log;
import java.util.concurrent.CountDownLatch;

/** Runs through upstream app_process/RuntimeInit, never a replacement Framework. */
public final class FrameworkMain {
    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] arguments) throws Exception {
        Parcel written = Parcel.obtain();
        Parcel read = Parcel.obtain();
        try {
            Bundle bundle = new Bundle();
            bundle.putString("name", "Bndroid Framework");
            bundle.putIntArray("values", new int[] {7, 11, 23});
            written.writeInt(0x424e4452);
            written.writeString("original native Parcel");
            written.writeBundle(bundle);
            byte[] bytes = written.marshall();
            require(bytes.length > 32, "empty Parcel encoding");
            read.unmarshall(bytes, 0, bytes.length);
            read.setDataPosition(0);
            require(read.readInt() == 0x424e4452, "Parcel integer");
            require("original native Parcel".equals(read.readString()), "Parcel string");
            Bundle restored = read.readBundle(FrameworkMain.class.getClassLoader());
            require("Bndroid Framework".equals(restored.getString("name")), "Bundle string");
            require(restored.getIntArray("values")[2] == 23, "Bundle array");
            require(read.dataAvail() == 0, "trailing Parcel data");
        } finally {
            read.recycle();
            written.recycle();
        }
        Log.i("bndroid-framework", "FRAMEWORK_STAGE parcel");
        Looper.prepare();
        long[] began = {0};
        Thread owner = Thread.currentThread();
        int[] calls = {0};
        Handler handler = new Handler(Looper.myLooper());
        CountDownLatch idle = new CountDownLatch(1);
        Looper.myQueue().addIdleHandler(() -> { idle.countDown(); return false; });
        Thread poster = new Thread(() -> {
            try { idle.await(); } catch (InterruptedException error) { throw new AssertionError(error); }
            SystemClock.sleep(50);
            require(handler.post(() -> {
                require(Thread.currentThread() == owner, "wake callback ownership");
                calls[0]++;
                Log.i("bndroid-framework", "FRAMEWORK_STAGE wake");
                began[0] = SystemClock.uptimeMillis();
                require(handler.postDelayed(() -> {
                    require(Thread.currentThread() == owner, "Looper thread ownership");
                    require(SystemClock.uptimeMillis() - began[0] >= 20, "early delayed message");
                    calls[0]++;
                    Looper.myLooper().quit();
                }, 20), "delayed message enqueue");
            }), "cross-thread message enqueue");
        }, "framework-poster");
        poster.start();
        Looper.loop();
        poster.join();
        require(calls[0] == 2, "Looper callback count");
        Log.i("bndroid-framework", "FRAMEWORK_STAGE looper");
        Log.i("bndroid-framework", "FRAMEWORK_MAIN_OK parcel=1 bundle=1 looper=1 delayed=1 wake=1");
    }
}
