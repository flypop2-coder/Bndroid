#!/usr/bin/env python3
"""Package verified local ART inputs and an owned DEX program for Bndroid."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import tarfile
import zipfile

from android_properties import encode_properties, runtime_properties
from android_framework_inputs import framework_inputs_complete, service_inputs_complete


def service_profiles(mode):
    manager = "u:object_r:service_manager_service:s0"
    runtime = "u:object_r:bndroid_runtime_service:s0"
    profiles = [dict(uid=1000, executable="servicemanager", args=[], binder_managers=1,
                     service_grants=[(3, manager), (4, "u:r:bndroid_profile_0:s0")],
                     property_grants=["servicemanager.ready"], control_grants=[])]
    if mode == "framework-bridge":
        profiles += [dict(uid=1001, executable="bndroid-runtime-service", args=[], binder_managers=0,
                          service_grants=[(2, manager), (3, runtime)], property_grants=[], control_grants=[]),
                     dict(uid=10000, executable="service-bridge-client", args=["client"], binder_managers=0,
                          service_grants=[(2, manager), (2, runtime)], property_grants=["bndroid.runtime.client_ready"], control_grants=[]),
                     dict(uid=1002, executable="service-bridge-client", args=["controller"], binder_managers=0,
                          service_grants=[(2, manager), (2, runtime)], property_grants=[], control_grants=[runtime])]
    else:
        profiles.append(dict(uid=10000, executable="launch-probe", args=["lookup" if mode == "framework-services" else "app"],
                             binder_managers=0, service_grants=[(2, manager)], property_grants=[], control_grants=[]))
        if mode == "framework-identities":
            profiles[0].update(executable="launch-probe", args=["system"])
    for index, profile in enumerate(profiles):
        profile.update(index=index, gid=profile["uid"], executable="/system/bin/" + profile["executable"])
    return profiles


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--dex", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--mode", choices=("java", "managed", "memory", "memfd", "binder", "poll", "properties", "help", "framework", "framework-help", "framework-identities", "framework-services", "framework-bridge", "signals", "signal-bad-frame", "signal-bad-pc", "signal-bad-fp", "signal-default", "signal-guard"), default="java")
    args = parser.parse_args()
    provenance = json.loads((args.runtime / "art-provenance.json").read_text())
    framework = args.mode.startswith("framework")
    if framework and "framework" not in provenance:
        raise SystemExit("Framework mode requires inputs extracted with --include-framework")
    if framework and not framework_inputs_complete(provenance):
        raise SystemExit("Framework metadata/JNI inputs are incomplete; re-run extraction with --resume-framework --include-framework")
    bridge = args.mode == "framework-bridge"
    service_mode = args.mode in ("framework-identities", "framework-services", "framework-bridge")
    if service_mode and not service_inputs_complete(provenance):
        raise SystemExit("Service mode requires verified inputs from extract-android-services.py")
    inputs = {}
    owned_inputs = {}
    for name, receipt in provenance["files"].items():
        path = PurePosixPath(name)
        if not name.startswith("/") or ".." in path.parts or str(path) != name:
            raise SystemExit(f"Noncanonical input path: {name}")
        data = (args.runtime / "root" / name.lstrip("/")).read_bytes()
        if len(data) != receipt["size"] or hashlib.sha256(data).hexdigest() != receipt["sha256"]:
            raise SystemExit(f"Runtime input changed since extraction: {name}")
        inputs[name.lstrip("/")] = data
    for name, receipt in provenance["source"]["files"].items():
        data = (args.runtime / name).read_bytes()
        if len(data) != receipt["size"] or hashlib.sha256(data).hexdigest() != receipt["sha256"]:
            raise SystemExit(f"Bionic input changed since extraction: {name}")
    # Use the source linker's real namespace policy. Old extraction receipts
    # must be refreshed: an unverified config or global LD_LIBRARY_PATH would
    # silently change which copies of APEX/platform libraries get loaded.
    metadata = [("ld.config.txt", "linkerconfig/ld.config.txt"),
                ("ld.art.config.txt", "linkerconfig/com.android.art/ld.config.txt")]
    if framework:
        metadata.append(("apex-info-list.xml", "apex/apex-info-list.xml"))
    for name, destination in metadata:
        receipt = provenance.get("metadata_files", {}).get(name)
        if receipt is None or receipt["path"] != "/" + destination:
            raise SystemExit("Re-extract ART inputs with namespace metadata using --include-art")
        data = (args.runtime / name).read_bytes()
        if len(data) != receipt["size"] or hashlib.sha256(data).hexdigest() != receipt["sha256"]:
            raise SystemExit(f"Linker configuration changed since extraction: {name}")
        inputs[destination] = data
    # System search paths reach the exact same Bionic files as the runtime APEX.
    # Resolve aliases inside the guest catalog, retaining canonical inode/path.
    aliases = {"system/lib64/" + name: "/apex/com.android.runtime/lib64/bionic/" + name
               for name in ("libc.so", "libm.so", "libdl.so", "libdl_android.so")}
    for name, destination in aliases.items():
        if destination.lstrip("/") not in inputs:
            raise SystemExit(f"Missing Bionic alias target: {destination}")
        if name in inputs:
            if inputs.pop(name) != inputs[destination.lstrip("/")]:
                raise SystemExit(f"Bionic alias differs from canonical input: {name}")
    inputs["system/framework/bndroid-probe.dex"] = args.dex.read_bytes()
    if framework:
        inputs["system/framework/bndroid-probe.dex"] = (args.dex.parent / "framework.dex").read_bytes()
    if framework or args.mode == "properties":
        properties = runtime_properties(provenance["source"])
        if service_mode:
            properties.update({"ro.property_service.version": "2", "servicemanager.ready": ""})
        mutable = ["servicemanager.ready"] if service_mode else []
        if bridge:
            properties["bndroid.runtime.client_ready"] = ""
            mutable.append("bndroid.runtime.client_ready")
        inputs["dev/__properties__"] = encode_properties(properties, mutable=mutable)
    if args.mode == "memfd":
        inputs["system/bin/memfd-probe"] = (args.dex.parent / "memfd-probe").read_bytes()
    if args.mode == "binder":
        inputs["system/bin/binder-probe"] = (args.dex.parent / "binder-probe").read_bytes()
    if args.mode == "poll":
        inputs["system/bin/poll-probe"] = (args.dex.parent / "poll-probe").read_bytes()
    if service_mode and not bridge:
        inputs["system/bin/launch-probe"] = (args.dex.parent / "launch-probe").read_bytes()
    if bridge:
        for name in ("bndroid-runtime-service", "service-bridge-client"):
            inputs["system/bin/" + name] = (args.dex.parent / name).read_bytes()
            owned_inputs["/system/bin/" + name] = hashlib.sha256(inputs["system/bin/" + name]).hexdigest()
        overlay = "product/etc/selinux/product_service_contexts"
        if overlay in inputs:
            raise SystemExit("Owned service-context overlay must not replace a verified source input")
        inputs[overlay] = b"bndroid.runtime u:object_r:bndroid_runtime_service:s0\n"
        owned_inputs["/" + overlay] = hashlib.sha256(inputs[overlay]).hexdigest()
    if args.mode == "properties":
        inputs["system/bin/properties-probe"] = (args.dex.parent / "properties-probe").read_bytes()
    if args.mode == "managed":
        inputs["system/framework/bndroid-probe.dex"] = (args.dex.parent / "managed.dex").read_bytes()
        inputs["system/lib64/libbndroidjni.so"] = (args.dex.parent / "libbndroidjni.so").read_bytes()
        inputs["data/app/bndroid.runtime/base.apk"] = (args.dex.parent / "managed.apk").read_bytes()
    # Bndroid's initial public native-library policy lists actual supplied
    # libraries. It does not advertise missing Android graphics/media APIs.
    inputs["system/etc/public.libraries.txt"] = b"libc.so\nlibdl.so\nlibm.so\nliblog.so\nlibz.so\n"
    if args.mode == "memory":
        inputs["system/bin/art-memory-probe"] = (args.dex.parent / "memory-probe").read_bytes()
    if args.mode.startswith("signal"):
        inputs["system/bin/art-signals-probe"] = (args.dex.parent / "signals-probe").read_bytes()
    if not inputs["system/framework/bndroid-probe.dex"].startswith(b"dex\n"):
        raise SystemExit("Expected a standard DEX input")
    selected_dex = inputs["system/framework/bndroid-probe.dex"]
    if args.mode == "managed":
        with zipfile.ZipFile(io.BytesIO(inputs["data/app/bndroid.runtime/base.apk"])) as apk:
            if apk.testzip() or len(apk.namelist()) != len(set(apk.namelist())):
                raise SystemExit("Corrupt or duplicate APK input")
            if apk.read("classes.dex") != selected_dex or apk.read("lib/arm64-v8a/libbndroidjni.so") != inputs["system/lib64/libbndroidjni.so"]:
                raise SystemExit("APK payload differs from the owned DEX/JNI build")
        # The runtime must actually load DEX from the APK. There is no loose
        # managed DEX or extracted application library on its positive path.
        del inputs["system/framework/bndroid-probe.dex"]
    if len(inputs) + len(aliases) + 4 + 10 > (512 if framework else 128):
        raise SystemExit("Rootfs exceeds catalog capacity")
    if any(len(name.encode()) > 100 for name in (*inputs, *aliases)):
        raise SystemExit("Runtime input exceeds the supported USTAR path field")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(args.output, "w", format=tarfile.USTAR_FORMAT) as archive:
        for name, data in sorted(inputs.items()):
            entry = tarfile.TarInfo(name)
            entry.size, entry.mode = len(data), 0o555
            archive.addfile(entry, io.BytesIO(data))
        for name, destination in sorted(aliases.items()):
            entry = tarfile.TarInfo(name)
            entry.type, entry.mode, entry.linkname = tarfile.SYMTYPE, 0o777, destination
            archive.addfile(entry)
        for name in ("data", "data/dalvik-cache", "system/etc", "apex/com.android.tzdata"):
            entry = tarfile.TarInfo(name)
            entry.type, entry.mode = tarfile.DIRTYPE, 0o555
            archive.addfile(entry)
    if args.output.stat().st_size > (320 if framework else 96) * 1024 * 1024:
        raise SystemExit("Rootfs exceeds the selected runtime profile's capacity")
    jars = ["/apex/com.android.art/javalib/" + name + ".jar" for name in
            ("core-oj", "core-libart", "okhttp", "bouncycastle", "apache-xml")]
    jars.append("/apex/com.android.i18n/javalib/core-icu4j.jar")
    arguments = ["/apex/com.android.runtime/bin/linker64", "/apex/com.android.art/bin/dalvikvm64"]
    arguments += ["-help"] if args.mode == "help" else ["-Xint", "-Xusejit:false", "-Xms4m", "-Xmx16m", "-Xnoimage-dex2oat", "-Xbootclasspath:" + ":".join(jars), "-cp", "/system/framework/bndroid-probe.dex", "Main"]
    if args.mode == "managed":
        arguments[-1] = "ManagedMain"
        arguments[-2] = "/data/app/bndroid.runtime/base.apk"
        arguments.insert(2, "-Djava.library.path=/data/app/bndroid.runtime/base.apk!/lib/arm64-v8a")
    if args.mode == "memory":
        arguments = [arguments[0], "/system/bin/art-memory-probe"]
    if args.mode == "memfd":
        arguments = [arguments[0], "/system/bin/memfd-probe"]
    if args.mode == "binder":
        arguments = [arguments[0], "/system/bin/binder-probe"]
    if args.mode == "poll":
        arguments = [arguments[0], "/system/bin/poll-probe"]
    if args.mode == "properties":
        arguments = [arguments[0], "/system/bin/properties-probe"]
    if args.mode.startswith("signal"):
        arguments = [arguments[0], "/system/bin/art-signals-probe", args.mode]
    environment = ["LANG=C", "ANDROID_ROOT=/system", "ANDROID_ART_ROOT=/apex/com.android.art",
                   "ANDROID_I18N_ROOT=/apex/com.android.i18n", "ANDROID_TZDATA_ROOT=/apex/com.android.tzdata",
                   "ANDROID_DATA=/data"]
    if framework:
        compiler_classpath = provenance["environment"]["DEX2OATBOOTCLASSPATH"]
        if any(path not in provenance["framework"]["bootclasspath"] for path in compiler_classpath.split(":")):
            raise SystemExit("Compiler boot classpath must use the verified source boot jars")
        arguments = [arguments[0], "/system/bin/app_process64"]
        arguments += ["-help"] if args.mode == "framework-help" else ["-Xint", "-Xusejit:false", "-Xms4m", "-Xmx16m", "-Xnoimage-dex2oat"]
        arguments += ["/system/bin", "--application", "--nice-name=bndroid.framework", "FrameworkMain"]
        environment += ["BOOTCLASSPATH=" + ":".join(provenance["framework"]["bootclasspath"]),
                        "DEX2OATBOOTCLASSPATH=" + compiler_classpath,
                        "CLASSPATH=/system/framework/bndroid-probe.dex"]
    launch = ["arg=" + s for s in arguments] + ["env=" + s for s in environment]
    if service_mode:
        launch = []
        profiles = service_profiles(args.mode)
        for profile in profiles:
            launch += [f"profile={profile['index']}", f"uid={profile['uid']}", f"gid={profile['gid']}",
                       f"binder_managers={profile['binder_managers']}",
                       "arg=/apex/com.android.runtime/bin/linker64", "arg=" + profile["executable"]]
            launch += ["arg=" + arg for arg in profile["args"]]
            launch += [f"service_grant={rights} {target}" for rights, target in profile["service_grants"]]
            launch += ["property_grant=" + name for name in profile["property_grants"]]
            launch += ["control_grant=" + target for target in profile["control_grants"]]
            launch += ["env=" + entry for entry in environment if not entry.startswith(("CLASSPATH=", "BOOTCLASSPATH=", "DEX2OATBOOTCLASSPATH="))]
    args.output.with_suffix(".launch").write_text("\n".join(launch) + "\n")
    receipt = {"source": provenance, "rootfs_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
               "dex_sha256": hashlib.sha256(selected_dex).hexdigest(), "mode": args.mode,
               "files": len(inputs) + len(aliases) + 4, "bytes": args.output.stat().st_size,
               "aliases": aliases, "namespace_mode": "verified source global and ART linker configuration"}
    if args.mode == "managed":
        receipt["apk_sha256"] = hashlib.sha256(inputs["data/app/bndroid.runtime/base.apk"]).hexdigest()
    if framework:
        receipt["entrypoint"] = "/system/bin/app_process64"
    if service_mode:
        receipt["entrypoint"] = profiles[0]["executable"]
        receipt["launch_profiles"] = profiles
    if owned_inputs:
        receipt["owned_inputs_sha256"] = owned_inputs
    if framework or args.mode == "properties":
        receipt["boot_properties"] = properties
    args.output.with_suffix(".json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"ART_ROOTFS_BUILD_OK files={receipt['files']} bytes={receipt['bytes']} path={args.output}")


if __name__ == "__main__":
    main()
