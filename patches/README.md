# Skyward 补丁集（m2481 / Android 16 底座）

基线：`un6666cn/skyward`（= kirklin/skyward 的魅族移植 fork，本目录的 `fork/` 快照）。
应用方式：在仓库根目录 `git apply`，或按文件手工合并。二进制文件（`*.patch`）按需取用。

## 必选（解密链路）

1. `crates_skyward-binder_src_parcel.rs.patch`
   **binder 调用头**。libbinder 写的头是
   `strict(0x80000000) + workSource(-1) + 0x53595354`（字节 `54 53 59 53`，即 "TSYS"）。
   原实现写 `"RECO"`，服务目录的 AIDL 分发因此认不出接口，**所有事务号都回
   `UNKNOWN_TRANSACTION`**（现象：`the call failed in the driver with error -2147483647`）。

2. `crates_skyward-binder_src_service.rs.patch`
   - Android 16 的 `checkService` 是 **4**（旧值是 3），`listServices` 是 6；
   - 16 的 `checkService` 回包把 binder 包在元数据里，旧 `read.object()` 读不到，
     改为在回包里扫 `flat_binder_object` 的类型字（`85 2a 68 73`）取 handle；
   - 查找时对多种编号做一次回退尝试，兼容不同底座。

3. `crates_skyward-crypto_src_keystore2.rs.patch`
   keystore2 的 `persistent.sqlite` 主体往往只有几 KB，数据都在 `-wal` 里。
   原实现用 `?immutable=1` 打开会**忽略 WAL** → `no such table: blobentry`。
   改为把 db + `-wal` 复制到 `/tmp` 后正常打开（打开即回放 WAL）。

## 可选（本机适配 / 体验）

4. `crates_skyward-crypto_src_data.rs.patch` + `crates_skyward_src_decrypt.rs.patch`
   「未加密 data 直接挂载」快速通道：探到 f2fs/ext4 就直接 rw 挂载并 bind `/sdcard`，
   不去碰 keymint。对刚格式化过（无加密）的 data 有用；有加密时自动回落到原流程。

5. `crates_skyward_src_ui.rs.patch`
   魅族面板的背光回退：读不到 `mi,*` 背光属性时，用
   `qcom,mdss-brightness-max-level` / `qcom,mdss-dsi-bl-max-level`，否则用满量程。
   （不修的话，m2481 上开机即黑屏并报
   `panel ... gives its normal maximum as none of [...]`。）

6. `crates_skyward-pack_src_main.rs.patch`
   packer 增强：`--extra <dir>`（把目录里的文件按路径塞进 ramdisk）、
   `--allow-custom`（允许以第三方 recovery 的 ramdisk 为底座，如 TWRP-16），
   以及链接冲突时先删同名真实文件。

7. `crates_skyward_src_main.rs.patch`
   仅为诊断用的 `service-dump` 子命令（打印服务目录对各事务号的回包），**不建议合入**。

## 设备配置

见 `devices/m2481-v14.toml`（本机最终配置）：
- `[firmware] partition = "modem"`（QSEE 读 TA，解密必需）
- `[partitions] mounted = ["persist"]`
- keymint / gatekeeper 用 `ready_service` 等待，keymint 以 **root** 启动
  （本机以 system 启动时注册不上）

另需在 ramdisk 里补一个 init 触发器（魅族 vendor rc 的 `on init` 在 recovery 的 init 里
不触发，keymint 永远不启动）：

```rc
# /system/etc/init/keymint-start.rc
on late-init
    start vendor.keymint-qti
```
