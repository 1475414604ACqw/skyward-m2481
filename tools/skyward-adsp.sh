#!/system/bin/sh
# 电量计固件在 ADSP 上：把 modem_a 的固件挂到 /vendor/firmware_mnt，
# 再按固件名找到 adsp 的 remoteproc 拉起来。ADSP 起来后 qti_battery_charger
# 才会注册 battery 电源，Skyward 才能读到电量/电流/温度。
mkdir -p /vendor/firmware_mnt
grep -q firmware_mnt /proc/mounts 2>/dev/null || \
    mount -t vfat -o ro /dev/block/bootdevice/by-name/modem_a /vendor/firmware_mnt 2>/dev/null
for d in /sys/class/remoteproc/remoteproc*; do
    [ "$(cat "$d/firmware" 2>/dev/null)" = "adsp.mdt" ] && echo start > "$d/state" 2>/dev/null
done
exit 0