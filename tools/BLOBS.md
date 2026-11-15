# The files the recovery lays over the TWRP ramdisk

`work/stage2` holds files the phone's own programs need, which the TWRP ramdisk
does not carry. They are **not** in this repository: they belong to the
installed system and to the vendor of the phone, and are not ours to ship.

Put them under `work/stage2`, keeping the paths they have under it.

## From the installed system's partitions

Mounted at `/mnt/system` (`system_a`) and `/vendor` (`vendor_a`) by the
recovery while it runs, or readable from Android with root:

| File | Path under `work/stage2` |
|---|---|
| `libion.so` | `system/lib64/libion.so` |
| `libdmabufheap.so` | `system/lib64/libdmabufheap.so` |
| `libwifi-system-iface.so` | `system/lib64/libwifi-system-iface.so` |
| `android.hardware.wifi-V1-ndk.so` | `system/lib64/` |
| `libwifi-hal.so`, `libcld80211.so`, `libnl.so` | `system/lib64/` |
| `android.hardware.wifi.supplicant-V2-ndk.so` | `system/lib64/` |
| `android.system.wifi.keystore@1.0.so` | `system/lib64/` |
| `libkeystore-*-wifi-hidl.so` | `system/lib64/` |
| `libcert_parse.wpa_s.so`, `libpasn.so` | `system/lib64/` |
| `vendor.qti.hardware.wifi.supplicant-V1-ndk.so` | `system/lib64/` |
| `libxml2.so` | `system/lib64/` |

## The BoringSSL the vendor supplicant was built against

The vendor `wpa_supplicant` needs Android 13/14's BoringSSL, which the
installed Android 16 system no longer ships (`/system/lib64/libcrypto.so` is
OpenSSL 3 and lacks `sk_dup`). The copy that works is in the VNDK apex:

```sh
adb root
adb pull /apex/com.android.vndk.v34/lib64/libcrypto.so
adb pull /apex/com.android.vndk.v34/lib64/libssl.so
```

Rename them so they do not shadow the system's own:

* `libcrypto.so` → `work/stage2/system/lib64/libc11.so`
* `libssl.so`   → `work/stage2/system/lib64/libs11.so`

and patch the names their consumers ask for, in place and with the same
length, with `tools/rename_openssl.py`:

```sh
python3 tools/rename_openssl.py work/stage2/system/lib64/libs11.so \
    work/stage2/system/bin/wpa_supplicant
```

The supplicant itself comes from the vendor partition
(`/vendor/bin/hw/wpa_supplicant`), copied to
`work/stage2/system/bin/wpa_supplicant`.

## Kernel modules

`rfkill.ko` is needed before `cfg80211` on this base:

```sh
adb pull /system_dlkm/lib/modules/6.1.25-android14-11-maybe-dirty/kernel/net/rfkill/rfkill.ko \
    work/stage2/lib/modules/rfkill.ko
```

## The services the configuration starts

`devices/meizu21pro.toml` names a few files the recovery puts in place for the
phone's own programs:

| File | Where it comes from |
|---|---|
| `system/bin/skyward-adsp.sh` | in this repository, copy it: `cp tools/skyward-adsp.sh work/stage2/system/bin/` |
| `system/etc/vintf/manifest/android.hardware.gatekeeper-service-qti.xml` | extracted from the vendor's `manifest.xml`, see below |

The vendor declares the gatekeeper service only inside its folded
`manifest_pineapple.xml`; the service directory accepts a hardware service only
where a separate fragment names it, so the `<hal>` block that describes
`android.hardware.gatekeeper` is cut out of the vendor manifest into a file of
its own, `android.hardware.gatekeeper-service-qti.xml`, under
`work/stage2/system/etc/vintf/manifest/`.
