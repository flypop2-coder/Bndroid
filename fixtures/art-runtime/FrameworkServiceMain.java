import android.os.Binder;
import android.os.DeadObjectException;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.os.Parcel;
import android.os.Process;
import android.os.SystemClock;
import android.system.Os;
import android.system.StructUtsname;
import android.util.Log;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.concurrent.atomic.AtomicInteger;
import org.bndroid.runtime.PlatformRuntime;

/** Platform integration through the original app_process and Framework classes. */
public final class FrameworkServiceMain {
    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static IBinder discover(Method check) throws Exception {
        for (int i = 0; i < 1000; i++) {
            IBinder binder = (IBinder) check.invoke(null, PlatformRuntime.SERVICE);
            if (binder != null && binder.pingBinder()) return binder;
            SystemClock.sleep(5);
        }
        throw new AssertionError("Runtime provider discovery timeout");
    }

    private static long snapshot(PlatformRuntime service) throws Exception {
        PlatformRuntime.Snapshot value = service.snapshot();
        require(value.uptimeNanos > 0 && value.uptimeNanos <= SystemClock.elapsedRealtimeNanos(), "uptime");
        require(value.providerPid != Process.myPid() && value.providerUid == 1001, "provider identity");
        require(value.callerPid == Process.myPid() && value.callerUid == Process.myUid(), "caller identity");
        StructUtsname system = Os.uname();
        require(value.hostname.equals(system.nodename) && value.abiRelease.equals(system.release)
                && value.machine.equals(system.machine), "kernel facts");
        return value.instance;
    }

    private static void malformed(IBinder binder, int version, boolean extra) throws Exception {
        Parcel request = Parcel.obtain(), reply = Parcel.obtain();
        try {
            request.writeInterfaceToken(PlatformRuntime.DESCRIPTOR);
            request.writeInt(version);
            if (extra) request.writeInt(99);
            require(binder.transact(1, request, reply, 0), "malformed transaction transport");
            try {
                reply.readException();
                throw new AssertionError("Malformed request accepted");
            } catch (IllegalArgumentException expected) {
                // Native AIDL exception header decoded by original Framework Parcel.
            }
        } finally {
            request.recycle();
            reply.recycle();
        }
    }

    public static void main(String[] arguments) throws Exception {
        require(Process.myUid() == 10000, "application identity");
        // Public SDK stubs omit these platform APIs. Reflection reaches the
        // original boot classes; no replacement classes or API exemptions.
        Class<?> manager = Class.forName("android.os.ServiceManager");
        Method check = manager.getMethod("checkService", String.class);
        Method add = manager.getMethod("addService", String.class, IBinder.class);
        Class<?> properties = Class.forName("android.os.SystemProperties");
        Method get = properties.getMethod("get", String.class);
        Method set = properties.getMethod("set", String.class, String.class);
        require("true".equals(get.invoke(null, "servicemanager.ready")), "original daemon readiness");
        IBinder old = discover(check);
        PlatformRuntime service = new PlatformRuntime(old);
        long first = snapshot(service);
        Log.i("bndroid-service", "FRAMEWORK_SERVICE_STAGE snapshot");
        try {
            add.invoke(null, PlatformRuntime.SERVICE, new Binder());
            throw new AssertionError("Application service replacement accepted");
        } catch (InvocationTargetException expected) {
            require(expected.getCause() instanceof SecurityException, "registration security exception");
        }
        try {
            service.requestStop();
            throw new AssertionError("Application maintenance accepted");
        } catch (SecurityException expected) {
            // The native provider enforces a separate control grant.
        }
        malformed(old, 99, false);
        malformed(old, 1, true);
        Log.i("bndroid-service", "FRAMEWORK_SERVICE_STAGE authority");
        Looper.prepare();
        Handler handler = new Handler(Looper.myLooper());
        Thread owner = Thread.currentThread();
        AtomicInteger deaths = new AtomicInteger();
        Thread[] callbackThread = {null};
        IBinder.DeathRecipient recipient = () -> {
            callbackThread[0] = Thread.currentThread();
            deaths.incrementAndGet();
            require(handler.post(() -> {
                require(Thread.currentThread() == owner, "death Looper ownership");
                Looper.myLooper().quit();
            }), "death handoff");
        };
        old.linkToDeath(recipient, 0);
        require(handler.postDelayed(() -> { throw new AssertionError("Service death timeout"); }, 10000), "timeout enqueue");
        set.invoke(null, "bndroid.runtime.client_ready", "true");
        Looper.loop();
        handler.removeCallbacksAndMessages(null);
        require(deaths.get() == 1 && callbackThread[0] != owner, "Binder worker death callback");
        require(!old.isBinderAlive() && !old.pingBinder(), "dead Binder state");
        try {
            service.snapshot();
            throw new AssertionError("Old service remained usable");
        } catch (DeadObjectException expected) {
            // A new provider must never revive this proxy.
        }
        Log.i("bndroid-service", "FRAMEWORK_SERVICE_STAGE death");
        IBinder replacement = discover(check);
        long second = snapshot(new PlatformRuntime(replacement));
        require(replacement != old && second != first && replacement.pingBinder(), "provider rediscovery");
        require(!old.pingBinder() && deaths.get() == 1, "stale reference after restart");
        old.unlinkToDeath(recipient, 0);
        Log.i("bndroid-service", "FRAMEWORK_SERVICE_CLIENT_OK pid=" + Process.myPid()
                + " uid=" + Process.myUid() + " first=" + Long.toUnsignedString(first)
                + " second=" + Long.toUnsignedString(second)
                + " facts=1 identity=1 add_denied=1 stop_denied=1 malformed=1 death=1 stale=1 rediscovery=1 daemon_ready=1 looper=1");
    }
}
