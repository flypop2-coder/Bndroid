package org.bndroid.runtime;

import android.os.BadParcelableException;
import android.os.IBinder;
import android.os.Parcel;
import android.os.RemoteException;

/** Versioned Java binding for Bndroid's native runtime provider. */
public final class PlatformRuntime {
    public static final String SERVICE = "bndroid.runtime";
    public static final String DESCRIPTOR = "org.bndroid.runtime.IPlatformRuntime";
    private static final int VERSION = 1;
    private final IBinder remote;

    public PlatformRuntime(IBinder remote) throws RemoteException {
        if (remote == null || !DESCRIPTOR.equals(remote.getInterfaceDescriptor())) {
            throw new IllegalArgumentException("Expected " + DESCRIPTOR);
        }
        this.remote = remote;
    }

    public IBinder asBinder() {
        return remote;
    }

    public Snapshot snapshot() throws RemoteException {
        Parcel reply = transact(1);
        try {
            if (reply.readInt() != VERSION) throw new BadParcelableException("Runtime version");
            Snapshot snapshot = new Snapshot(reply);
            if (reply.dataAvail() != 0) throw new BadParcelableException("Trailing runtime data");
            return snapshot;
        } finally {
            reply.recycle();
        }
    }

    /** Requires a separate supervisor-issued maintenance grant. */
    public void requestStop() throws RemoteException {
        Parcel reply = transact(2);
        try {
            if (reply.dataAvail() != 0) throw new BadParcelableException("Trailing stop reply");
        } finally {
            reply.recycle();
        }
    }

    private Parcel transact(int code) throws RemoteException {
        Parcel request = Parcel.obtain();
        Parcel reply = Parcel.obtain();
        boolean returned = false;
        try {
            request.writeInterfaceToken(DESCRIPTOR);
            request.writeInt(VERSION);
            if (!remote.transact(code, request, reply, 0)) {
                throw new RemoteException("Unsupported runtime transaction " + code);
            }
            reply.readException();
            returned = true;
            return reply;
        } finally {
            request.recycle();
            if (!returned) reply.recycle();
        }
    }

    public static final class Snapshot {
        /** An unsigned 64-bit instance identifier; changes on provider restart. */
        public final long instance;
        public final long uptimeNanos;
        public final int providerPid, providerUid, callerPid, callerUid;
        public final String hostname, abiRelease, machine;

        private Snapshot(Parcel reply) {
            instance = reply.readLong();
            uptimeNanos = reply.readLong();
            providerPid = reply.readInt();
            providerUid = reply.readInt();
            callerPid = reply.readInt();
            callerUid = reply.readInt();
            hostname = reply.readString();
            abiRelease = reply.readString();
            machine = reply.readString();
            if (instance == 0 || uptimeNanos < 0 || providerPid <= 0 || providerUid < 0
                    || callerPid <= 0 || callerUid < 0 || hostname == null
                    || abiRelease == null || machine == null) {
                throw new BadParcelableException("Invalid runtime snapshot");
            }
        }
    }
}
