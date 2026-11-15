# Skyward for MEIZU 21 Pro (m2481 / meizu21pro)

A port of [Skyward](https://github.com/kirklin/skyward) 0.3.1 recovery to the
MEIZU 21 Pro (`m2481`, SM8650, 1368×3192, A/B slots, Flyme 12 / Android 16).

This tree is upstream Skyward 0.3.1 with the patches below. It is **not** a
stock Skyward build: the recovery is packed onto the TWRP-16 ramdisk, because
that is the only Android 16 userland this phone has.

## What works

| Feature | State |
|---|---|
| Data decryption (metadata + FBE users) | works |
| WiFi (scan / join / DHCP / internet) | works |
| Vibration (clicks, power-key ticks) | works, through the motor's LED interface |
| Battery / temperature | works (ADSP brought up by a oneshot service) |
| AVB page (read / write vbmeta flags) | works |
| Flash / backup / restore / mount | works |
| ADB shell / MTP | works |

Not in this version: the browser page and the screen lock of earlier ports.

## Patches over upstream

1. **binder call header** — libbinder on this base reads `TSYS`
   (`0x53595354`), not upstream's `RECO`, or every transaction returns
   `UNKNOWN_TRANSACTION`.
2. **service directory numbering** — Android 16's `checkService` is 4 here;
   `find()` tries several numberings and scans the reply for the wrapped
   `flat_binder_object`.
3. **key service database** — `persistent.sqlite` is copied together with its
   `-wal` log before it is opened (opening it immutable ignores the log).
4. **unencrypted data fast path** — a data partition a recovery formatted is
   mounted directly, without asking the secure world.
5. **packer** — `--extra <dir>` adds files to the ramdisk, `--allow-custom`
   accepts another recovery's ramdisk as the base.
6. **WLAN cold-boot calibration** — after the driver settles, the modem
   firmware partition is mounted at `/vendor/firmware_mnt` and
   `/sys/kernel/cnss/fs_ready` is written; the chip then calibrates in ~6 s.
7. **backlight fallback** — Meizu panels name none of Xiaomi's brightness
   levels; the display driver's own maximum is used instead.
8. **haptics** — the AAC RT6010 driver accepts force-feedback effects but
   produces no motion; clicks are written to its LED interface
   (`duration` + `activate`) instead.
9. **haptics module timing** — `haptic_aac.ko` must not be loaded by init
   before `/vendor` and `/persist` are mounted, or its f0 calibration fails
   (`get slot cali data: -19`) and the amplifier stays silent. The module is
   dropped from `lib/modules/modules.load.recovery` and loaded by Skyward
   after the partitions are mounted.

See `docs/BUILD_m2481.md` for the full build log, failures and measurements.

## Building

Requirements: Rust, Android NDK r29, a Linux host or WSL for the packer.

```sh
export ANDROID_NDK_HOME=/path/to/android-ndk-r29
cargo ndk -t arm64-v8a --platform 30 build --release -p skyward -p skyward-system-init
cargo build --release -p skyward-pack
```

Packing needs:

* a TWRP-16 recovery image for the phone (`--stock`), which is the base the
  recovery is laid over;
* the files in `work/stage2` (libraries the phone's own programs need), see
  `tools/BLOBS.md`;
* the fonts, which are in `resources/fonts`.

```sh
STOCK=/path/to/TWRP-16.img tools/pack.sh devices/meizu21pro.toml out/skyward.img
```

Flash both recovery slots:

```sh
fastboot flash recovery_a out/skyward.img
fastboot flash recovery_b out/skyward.img
fastboot reboot recovery
```

## License

GPL-3.0, as upstream Skyward. See `LICENSE`.
