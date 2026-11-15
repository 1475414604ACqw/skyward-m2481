# 给 21 Pro（m2481）编 Skyward：输入、步骤、还缺什么

> 代码已拉到本地：[src/](src)（`main`，288 个文件）
> 设备配置草稿已写：[devices/m2481.toml](devices/m2481.toml)（TOML 语法已校验通过）
> **2026-11-11 22:55 已编出成品**：[out/skyward-m2481.img](out/skyward-m2481.img)（100 MB）
> 生成时间：2026-11-11

## 0. 实操结果（本机真实跑通）

| 阶段 | 结果 |
|---|---|
| Rust 工具链 | `rustup` + `stable-x86_64-pc-windows-gnu` 1.99.1（GNU host，免装 VS） |
| Android NDK | r29，解到 `D:\MyDrivers\toolchains\android-ndk-r29`（2.34 GB） |
| host 链接器 | 补了 WinLibs MinGW-w64（rustup 自带 dlltool 跑不起来） |
| `skyward`（aarch64） | ✅ 8.2 MB，ELF64 / machine 183 (AArch64) |
| `system-init`（静态 aarch64） | ✅ 0.99 MB |
| `skyward-pack` | ⚠️ **只能在 Linux 上编**（用了 `std::os::unix`），改在 WSL（`MeizuBuild` / Ubuntu 24.04）里编，4.1 MB |
| 打包 | ✅ `skyward-m2481.img` 104,857,600 B，ramdisk 38,570,494 B |
| SHA-256 | `b4c25c44e721ee4a6fd71f06145dbcdf51121e4b7772d23b94039a30aa995030` |
| 镜像内已确认 | `skyward/system-init`、`skyward/device.toml`、两套字体、改写过的 `prop.default`、`system/etc/init/skyward-mtp.rc` |

**踩到的坑（都已绕开）：**
1. Windows 上 `cargo build`（GNU host）报 `dlltool.exe not found` → 换 WinLibs 的 binutils；
2. `skyward-pack` 依赖 unix 专有 API，Windows 编不过 → 用 WSL 编；
3. 配置里 `[stock].package` 会被校验：必须 `https://` + `.zip` + 文件名含 version；本地包对不上 → **打包时把 `[stock]` 整段注释掉**（`recovery` 子命令不需要它）。


## 1. 已经拿到的输入

| 输入 | 状态 | 来源 |
|---|---|---|
| 设备配置 `devices/m2481.toml` | ✅ 草稿（4 个服务 + 显示 + 备份 + stock 已填） | 从本机 `recovery.img` 的 ramdisk 里读出的 `prop.default` / `recovery.fstab`，加 TWRP 设备树的 vendor 服务定义 |
| stock recovery 镜像 | ✅ 就用 `flyme12.0.6.0A/images/recovery.img` | 100 MB，header v4，kernel 段为空，ramdisk 17.4 MB（legacy lz4） |
| recovery SHA-256 | ✅ `3dbdf82314e4b3178136aa3f03eb6ec1d54ee0f602539f0ba905edfd838ddab4` | 本地实测，已写进 `[stock]` |
| 字体 | ✅ `resources/fonts/MiSansVF.ttf`(19 MB) / `KirkMono-Regular.ttf` / `KirkMono-OFL.txt` | 仓库自带 |
| 设备关键属性 | ✅ `ro.product.mobile.name=m2481`、`MEIZU 21 Pro`、`pineapple`、Android 14 / SDK 34、`Flyme 12.26.1.7 daily` | ramdisk `prop.default` |

## 2. 还缺两样

**① 本机没有构建工具链**：`cargo` / `rustc` / `clang` / Android NDK 全都没有（已确认）。
Skyward 要 aarch64-linux-android 目标 + NDK r29；`skyward-pack` 可以在 Windows 上原生编译。

**② 设备侧 5 个字段没核实**（手机现在不在同一网段，MCP 连不上）：

| 字段 | 现在的值 | 怎么确认 |
|---|---|---|
| `display.density` | 480 | 读 `ro.sf.lcd_density` 或 `wm density` |
| `display.corner_radius` | 80 | 截图量圆角像素 |
| `modules.load`（触摸） | 空 | `ls /vendor/lib/modules` 找 goodix/meizu 触摸 ko |
| `wlan.*`（整段注释掉了） | — | `ls /vendor/lib/modules | grep -i wlan`，SM8650 多为 `qca_cld3_*` |
| `wipe.logs` | `mzlog` 等 | `ls /data` 看 Flyme 日志目录 |

## 3. 本机构建步骤（Windows/Linux 都行）

```bash
# ① Rust + 目标
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add aarch64-linux-android
cargo install cargo-ndk

# ② Android NDK r29（解压后设环境变量）
export ANDROID_NDK_HOME=/path/to/android-ndk-r29

# ③ 两个上机产物
cargo ndk -t arm64-v8a --platform 30 build --release -p skyward
RUSTFLAGS="-C target-feature=+crt-static" \
  cargo ndk -t arm64-v8a --platform 30 build --release \
  -p skyward-system-init --target-dir target/system-init

# ④ 打包器（跑在电脑上）
cargo build --release -p skyward-pack
```

## 4. 打包成镜像

```bash
# 直接用本机的 stock recovery（不必走 stock 子命令，那条要官网直链）
target/release/skyward-pack recovery \
  --stock    "C:/Users/VBVC6/Desktop/flyme12.0.6.0A/images/recovery.img" \
  --binary   target/aarch64-linux-android/release/skyward \
  --system-init target/system-init/aarch64-linux-android/release/skyward-system-init \
  --config   devices/m2481.toml \
  --font     resources/fonts/MiSansVF.ttf \
  --mono     resources/fonts/KirkMono-Regular.ttf \
  --mono-license resources/fonts/KirkMono-OFL.txt \
  --out      target/skyward-m2481.img
```

- 输出会**填满 100 MB 的 recovery 分区**并自带未签名 AVB footer；
- 打包器会拒绝「已经带 `twres`/`FFiles` 的镜像」，也就是**不能拿 TWRP 当底座**，必须用原厂 recovery。

刷入（A/B 两槽都要刷）：

```bat
fastboot flash recovery_a target/skyward-m2481.img
fastboot flash recovery_b target/skyward-m2481.img
fastboot reboot recovery
```

## 5. 不装工具链的替代路线：GitHub Actions

参考工作流见 [work/build-skyward.yml](work/build-skyward.yml)：fork 一份 Skyward → 改 `devices/m2481.toml` → 跑 Actions → 下两个 aarch64 产物，再在本机跑第 4 步（打包只要 `skyward-pack`，Windows 上 `cargo build` 即可，不需要 NDK）。

## 6. 风险

- 设备配置里 `[[service]]` 的 `ready_*` 判据是从 TWRP 的 rc 推的，**没在真机上验证过**；写错的直接表现是「卡在 Skyward logo 或没有振动/解不了密」。
- 触摸模块名不对 = 进去点不动，只能靠音量键/按键操作或重刷原厂 recovery。
- 结论：**先做出来、先用 `fastboot boot` 临时启动试**（不写分区），能用再 `flash`。

## 7. 第一次测试：`fastboot boot` 回原系统（2026-11-11 22:58）

- 现象：`fastboot boot skyward-m2481.img` → 直接重启回原系统。
- 原因：这台机的 recovery 分区镜像**不含内核**（`recovery.img` header v4，kernel_size=0），内核在 `boot.img` 里（33,892,864 B，且 boot 不带 ramdisk）。`fastboot boot` 拿到一个没内核的镜像，只能回落到正常开机。
- 对策：
  1. 用 `skyward-pack boot` 把 `boot.img` 的内核拼进 Skyward 的 ramdisk，得到 **`out/skyward-m2481-testboot.img`**（72,470,528 B），再 `fastboot boot` 它；
  2. 或者走正规路径 `fastboot flash recovery_a/b skyward-m2481.img` + `fastboot reboot recovery`（recovery 分区本来就是「只有 ramdisk、内核取自 boot」的设计）。
## 8. 第二次：刷 recovery 后黑屏重启（2026-11-11 23:10）

- 现象：刷 `skyward-m2481.img` 到 recovery + `fastboot reboot recovery` → **黑屏，然后自动重启**。
- 含义：recovery 内核和 ramdisk 都被加载了（否则会直接进系统），但用户态的 `recovery()` 提前返回错误，init 把失败的服务重启/整机重启。
- 源码里的落点（`crates/skyward/src/main.rs:762`）：
  ```rust
  run().inspect_err(|e| { let _ = skyward_sys::kmsg(&format!("recovery failed: {e:#}")); })
  ```
  → **真正的错误信息在 kernel log 里**（`recovery failed: ...`）。
- 取日志的两条路：
  1. 黑屏期间 `adb devices`；Skyward 在启动早期就 `set_prop sys.usb.config adb`，如果 adbd 活着就能 `adb shell dmesg | grep -i skyward`；
  2. 重启进系统后读 `adb shell su -c "cat /sys/fs/pstore/console-ramoops-0 | grep -i skyward"`。
- 同时做了**最小配置对照版**（去掉所有 `[[service]]` / `[haptics]` / `[backup]` / `[wipe]`，只留 device+display）：`out/skyward-m2481-min.img`，sha256 `7df7bb35…4c4dda`。
  - 它能进 UI → 问题在我写的服务条目，逐个加回；
  - 它同样黑屏重启 → 问题在更底层（显示/DRM 或内核），需要上面的日志。

## 9. ✅ 第三次：进去了（2026-11-12 06:48）

用 recovery 里的 root adb 抓到根因（`dmesg`）：

```
skyward: recovery failed: panel qcom,mdss_dsi_meizu_amoled_m2481_cmd gives its normal
maximum as none of [("mi,normal-max-brightness-clone","mi,max-brightness-clone"),
("mi,max-brightness-normal","qcom,mdss-dsi-bl-max-level"),
("mi,mdss-dsi-fac-bl-max-level","qcom,mdss-dsi-bl-max-level")]
```

→ Skyward 的 `normal_max()` 只认小米面板的 `mi,*` 亮度属性，魅族面板一个都没有 → 直接报错 → `ui::run` 失败 → 进程退出 → init 重启 → **黑屏循环**。

**补丁**（`crates/skyward/src/ui.rs`）：

- `/sys/module/msm_drm/parameters/dsi_display0` 读不到 → 直接用背光满量程；
- 面板节点没有 `mi,*` → 退回 `qcom,mdss-brightness-max-level` / `qcom,mdss-dsi-bl-max-level`（实测 4095），正常最大 = 满量程。

**实测设备数据**（recovery 内 root adb）：

| 项 | 值 |
|---|---|
| 面板 | `qcom,mdss_dsi_meizu_amoled_m2481_cmd` |
| 背光 | `/sys/class/backlight/panel0-backlight`，max 4095，当时 3515 |
| 面板亮度属性 | `qcom,mdss-dsi-bl-max-level`=4095、`-bl-min-level`=1、`qcom,mdss-brightness-max-level`=4095、`qcom,mdss-brightness-default-level`=1471 |
| 触摸驱动 | **`goodix_ts.ko`**（已写进 `[modules].load`） |
| WLAN 驱动 | `qca_cld3_kiwi_v2.ko` + `cnss*.ko`（无 `rfkill.ko`） |
| 已挂载 | odm/system/product/system_ext/vendor/vendor_dlkm/system_dlkm（全 erofs） |
| 已装系统 | Android 16，安全补丁 2025-12-05，Flyme 12.6.0.0A（Skyward 首页显示） |

**成品**：`out/skyward-m2481-v2.img`，sha256 `8de91d55…c7dec1c1`（100 MB），两槽已刷入，开机进 Skyward 主界面。

## 10. v3：加回解密/重启所需服务（2026-11-12 07:10）

- 配置：`devices/m2481-v3.toml` —— 在 v2 基础上加回 4 个原厂服务（qseecomd / keymint / gatekeeper / boot-service），**不带 `ready_*` 等待**，避免属性对不上时卡 20 秒再重启。
- 产物：`out/skyward-m2481-v3.img`（100 MB），sha256 `57e87343…7753e67e`。
- 风险：`services::start()` 在程序不存在时会 `bail("{} is missing; is its partition mounted?")`，直接导致回收重启；若发生，dmesg 里会点名是哪个路径。
- 回退：v2（`out/skyward-m2481-v2.img`）已验证可进主界面。

## 11. v3 实测：能进 UI，触摸在，解密卡在库

- **v3 正常进入主界面**（无 `recovery failed`），`qseecomd` 与 `boot-service` 都能起来。
- **触摸可用**：加载 `goodix_ts` 后出现 `/dev/input/event5`，名字 `main_touch`；另外 `aac-haptics`、`qcom-hv-haptics-dis` 也在。
- **解密失败**，界面报：
  `android.hardware.security.keymint.IKeyMintDevice/default is not running; the phone's key service has to be started first`
  `dmesg` 里 keymint / gatekeeper 每秒重启一次，`exit status: 1`。

**根因（用 adb 手动复现得到）**：

```
CANNOT LINK EXECUTABLE ".../keymint-service-qti": library "libhardware.so" not found
```

魅族 recovery 的 `/system/lib64` 只有 69 个库，没有 `libhardware.so`；装上之后依赖链继续往下：

```
libhardware.so (system) → libvndksupport.so (system) → libdl_android.so (system)
                       → libdmabufheap.so (system) → 找不到 libc++ 符号
                         _ZNSt3__122__libcpp_verbose_abortEPKcz
```

即：**系统分区的库是用系统自己的 libc++ 编的，recovery 用的是 ramdisk 里的 libc++，两者 ABI 对不上**。Skyward 只挂 VNDK apex（`/apex/com.android.vndk.v34`），没挂 `com.android.runtime`，所以拿不到系统的 libc++。

**可选修法（按工作量排序）**：

1. 给 Skyward 打补丁：启动时把 `com.android.runtime.apex` 也挂到 `/apex/com.android.runtime`，并把这几个目录放进服务的 `LD_LIBRARY_PATH`（`/apex/com.android.runtime/lib64`、`/mnt/system/system/lib64`、`/vendor/lib64`）；
2. 学 TWRP：把 keymint/gatekeeper 需要的库直接打进 recovery ramdisk（需要给 `skyward-pack` 加 `--extra` 之类的能力）；
3. 放弃 Skyward 内置解密，需要读 /data 时用 TWRP。

## 12. 解密攻关记录：卡在 linker namespace（2026-11-12 07:0x）

实测结论（都在 recovery 里用 root adb 验证）：

1. keymint / gatekeeper 换不到库，不是因为路径没给，而是**链接器命名空间限制**：
   - 真正生效的配置是 **`/linkerconfig/ld.config.txt`**（每次开机由 init 生成，356 B），里面只有：
     ```
     dir.recovery = /system/bin
     [recovery]
     namespace.default.isolated = false
     namespace.default.search.paths = /system/${LIB}
     ```
   - 改 `/system/etc/ld.config.txt` **无效**（链接器优先读 /linkerconfig）。
   - 往 /linkerconfig 里 `search.paths +=` 两个目录也**无效**（libhardware 仍找不到）。
   - 加 `permitted.paths` 后**搞坏了配置**：连 `cat`/`head`/`sh` 都报 `libc.so not found`（动态链接的程序全废），只能靠重新刷镜像恢复。
2. 反过来，**把 .so 直接拷进 `/system/lib64` 是有效的**（libhardware → libvndksupport → libdl_android → libdmabufheap 一层层都能过），直到 `libdmabufheap.so` 需要新 libc++ 的符号 `_ZNSt3__122__libcpp_verbose_abortEPKcz` 才断——这就是 ABI 不一致。
3. 拷 `libc++_shared.so` 等系统库进去时，曾导致 recovery 掉线重启一次（可能覆盖了正在 mmap 的库）。

**结论**：Skyward 的 `[service].libraries` 只能追加 LD_LIBRARY_PATH，绕不开 /linkerconfig 的命名空间；要让 keymint 跑起来，只有两条真正可行的路：

- **B（推荐）**：把 keymint/gatekeeper 依赖的 .so 打进 recovery ramdisk 的 `/system/lib64`（TWRP 就是这么干的），链接器天然认；
- **D**：改 `/linkerconfig/ld.config.txt` 的生成源（init 的 ld.config 模板），让 recovery 的默认命名空间包含系统/VNDK 库目录 —— 要动 ramdisk 里的 init 或它的模板，风险高于 B。

**当前机器状态（供恢复）**：v3 镜像在 `out/skyward-m2481-v3.img`；若 adb shell 报 `libc.so not found`，说明 RAM 里的 /linkerconfig 被改坏，`fastboot flash recovery_a/b` 重刷 v3 即可恢复（ramdisk 每次启动重建）。

## 13. v4：把依赖库打进 ramdisk（方案 B）

- `skyward-pack` 加了 `--extra <目录>`：把该目录整棵树按同名路径注入 ramdisk（已存在则替换），实现见 `crates/skyward-pack/src/main.rs` 的 `add_tree()`。
- 依赖闭包用 `work/collect_libs.py` 自动收集：从 4 个 HAL 二进制出发解析 ELF 的 `DT_NEEDED`，逐个 `adb pull`（不需要 shell，所以 recovery 链接器坏着也能干活），跳过 ramdisk 里已有的库，直到闭合。
- 结果：**29 个 .so（8.6 MB）**，含 `libhardware / libvndksupport / libdl_android / libdmabufheap / libqtigatekeeper / libqtikeymint / libQSEEComAPI / libkeymaster* / libxml2 / libandroidicu + ICU`，与 ramdisk 原有 70 个库**无重名**。
- `libandroidicu.so` 不在任何已挂载分区里，是从 `com.android.i18n.apex` 里取的（WSL 里 `mount -o loop` 挂载 payload 提取）。
- 产物：`out/skyward-m2481-v4.img`，ramdisk 42.4 MB，sha256 `730424ef…66aa2e`。
- 配置 `devices/m2481-v4.toml`：服务 `libraries = ["/system/lib64", "/vendor/lib64", "/vendor/lib64/hw"]`。

## 14. v4 失败：注入系统库会弄坏 DRM

- 注入 29 个库（ramdisk `/system/lib64` 从 69 → 98 个）后，UI 起不来，`dmesg`：
  ```
  skyward: recovery failed: set the panel mode: atomic commit: Permission denied (os error 13)
  ```
  → 黑屏循环（库把 DRM/GBM 路径带坏了，最可能是我带进 `libdmabufheap.so` 这类系统版本）。
- 就算 UI 能起，keymint 仍报缺 `_ZNSt3__122__libcpp_verbose_abortEPKcz`——因为系统库要**系统自己的 libc++**，而 ramdisk 里是旧版 `libc++.so`（缺新符号）。要修还得把系统 `libc++.so` 也塞进去，那会威胁 recovery 自己的一堆程序。
- **结论**：Skyward 在魅族上做解密，需要的是 TWRP 那种「整套库 + 隔离」方案，不是往 `/system/lib64` 里塞几个 .so 能解决的。已回滚到 v3。

## 当前状态（2026-11-12 07:20）

- 设备已回刷 **v3**：主界面 ✅、触摸 ✅、挂载 ✅、重启到 fastboot ✅、解密 ❌（keymint 每秒重启，属已知、不影响其它功能）。
- v3：`out/skyward-m2481-v3.img`（sha256 `57e87343…7753e67e`）
- v4（实验）：`out/skyward-m2481-v4.img`（sha256 `730424ef…66aa2e`）—— 不要刷，会黑屏。

## 15. 补充：v4 其实能开机，且解密的"最后一层"是 libbinder_ndk

- 复核：v4 刷入后主界面能起（我先前看到的 `set the panel mode: Permission denied` 是启动早期的一次失败重试，不是最终状态）。
- 解密仍失败。在 v4 上继续追依赖，卡点变成：
  ```
  cannot locate symbol "AIBinder_Class_setTransactionCodeToFunctionNameMap"
  referenced by android.hardware.security.rkp-V3-ndk.so
  ```
  → recovery 自带的 `libbinder_ndk.so` 比系统 NDK 库老。继续修就得把系统的 `libbinder_ndk / libbinder / libutils / libbase…` 一整套都塞进来，**每补一个就冒下一个**。
- 这正是 TWRP 那套设备树要带几十个 .so + 自己的 init/ld 配置的原因。
- **结论**：Skyward 在 m2481 上要解密 = 复刻 TWRP 的库栈，投入产出不划算；/data 交给 TWRP，Skyward 负责刷写/备份/切换。

## 16. v5：解密依赖打通了，但动了 recovery 自己的核心库

- 在运行中的 v4 上逐个 push 试出来：给 keymint/gatekeeper 补到 **47 个 .so** 后，三个服务全部能常驻（qseecomd ✅ keymint ✅ gatekeeper ✅，boot-service 仍会退出）。
- 其中 **12 个与 ramdisk 原有库重名**（libbase / libbinder / libbinder_ndk / libc++.so / libcutils / libhidlbase / liblog / libpackagelistparser / libprocessgroup / libselinux / libutils / libz）——它们是 Android 16 版本，覆盖了 recovery 的 Android 14 版本。
- 打成 v5 刷入后：**recovery 没起来**（adb 不出现），因为 init/adbd 也链接这些被覆盖的核心库，A16 库要 A14 libc 里没有的符号。
- 产物：`out/skyward-m2481-v5.img`（sha256 `c043d2cc…`）—— **不要刷**。
- 结论回到根因：**stock recovery 是 Android 14，装的系统是 Android 16**，vendor HAL 跑不进 A14 用户态。要解密只能在 A16 底座上做（TWRP-16 就是），Skyward 这条线到头了。

## 17. ✅ 突破：换到 A16 底座（TWRP-16 的 ramdisk）后，keymint/gatekeeper 全跑起来了

- 思路：`skyward-pack` 需要一个「原厂 recovery」当底座；魅族现成的 A14 recovery 跑不了 A16 的 vendor HAL，于是改用 **TWRP-16 的 ramdisk（AOSP 16 底座、同机型 m2481）**。
- 为此给 packer 加了两处能力：
  1. `--allow-custom`：允许在带第三方树（twres/FFiles）的 ramdisk 上打包；
  2. `Ramdisk::remove()` + 链接前清理：把 TWRP 自带的 `7za` 等真实文件替换成指向 Skyward 的链接。
- 在 TWRP 底座上实测依赖：keymint 只差 **2 个库**（`libion.so`、`libdmabufheap.so`，取自已装系统），gatekeeper 0 个。
- v7 = TWRP 底座 + Skyward + 这 2 个库：
  - 进程：`recovery`(Skyward) ✅、`hwservicemanager` ✅、`qseecomd` ✅、**`keymint` ✅**、**`gatekeeper` ✅**
  - `dmesg` 里不再有 "the data partition stays closed"
  - sha256 `ba00dd9e…df46f6`
- 关键结论：**解密能不能做，取决于 recovery 底座的 Android 版本，不是靠补库**。A14 底座无解，A16 底座只要 2 个库。

## 18. ✅ 解密打通：去掉 Skyward 在 parcel 里写的私有头

- A16 底座下 keymint/gatekeeper 已常驻，但 Skyward 查服务时报 `the call failed in the driver with error -2147483647`（FAILED_TRANSACTION）。
- 根因：`crates/skyward-binder/src/parcel.rs` 的 `Parcel::call()` 会在 strict-mode/work-source 之后写一个私有标记 `"RECO"`（`HEADER`），用来区分它自己那套 libbinder；而系统自带的 `servicemanager` 不认识这个 i32，直接把接口名当长度读 → `onTransact` 返回 false → FAILED_TRANSACTION。
- 改法：默认不写这个头（`SKYWARD_HEADER=1` 时才写），并给 `service.rs` 加了事务号回退（新旧两种编号都试）。
- 结果：`dmesg` 里不再出现 `the data partition stays closed`，keymint/gatekeeper 常驻，等待输入锁屏凭据。
- 镜像：`out/skyward-m2481-v9.img`（sha256 `19b09bb6…`）

## 19. 卡点：A16 的 servicemanager 改了协议

- 现象：A16 底座上 keymint/gatekeeper 常驻，但 Skyward 查服务时四种事务编号全被拒：
  `the directory turned every numbering away ([3, 2, 1, 4])`
- 原因（查 rsbinder 的 A16 实现确认）：Android 16 的 `IServiceManager` 新增了
  `getService2` / `checkService2`，返回值从"裸 binder"变成带元数据的 `Service` 联合体
  （`ServiceWithMetadata` 或 `IAccessor`）。Skyward 的 raw binder 客户端是老协议解析，
  直接失败。
- 要修必须照 A16 的 AIDL 重写 `skyward-binder` 的服务查询与 union 解析（`service.rs` 约 40~60 行 +
  `ServiceWithMetadata`/`Accessor` 的 marshaling），属于改上游代码。
- 状态：最后一次带调试日志的构建启动挂掉，设备 adb/fastboot 均失联，需手动长按电源再进 fastboot，
  刷回 `out/skyward-m2481-v4.img`（A14 底座，能开机，无解密）或 `skyward-m2481-v9.img`
  （A16 底座，crypto 服务在，解密仍不通）。

## 20. 结论：A16 底座上，Skyward 的 raw binder 客户端根本不被 servicemanager 接受

- 带日志的版本把每个事务号的结果都写进了 `/tmp/sm-debug.txt`：
  ```
  code 3: transact failed: the call failed in the driver with error -2147483647
  code 2: transact failed: ...
  code 1: transact failed: ...
  code 4: transact failed: ...
  ```
  → **失败发生在事务层**（`FAILED_TRANSACTION`），不是"方法号不对"。说明 A16 的
  `servicemanager` 直接拒收 Skyward 拼的 parcel / 接口令牌。
- A16 的 `IServiceManager` 已改成 `getService2`/`checkService2` 返回 `Service` 联合体
  （`ServiceWithMetadata` / `IAccessor`）。要接上，必须按 A16 的 AIDL 重写
  `skyward-binder` 的 parcel 头、方法号与 union 解析——是"改上游协议层"，不是配置能解决的。
- 因此本轮到此：**TWRP 底座 + Skyward** 已经能开机、触摸、挂载、刷写、备份、重启 fastboot，
  crypto 服务也能常驻；只有"通过 Skyward 解密 /data"这一步受协议层阻断。
- 最终留在机器上的版本：`out/skyward-m2481-v7.img`（sha256 `ba00dd9e…`）。

## 21. binder 层实验结论（2026-11-12 13:1x）

给 `skyward` 加了个 `service-dump` 子命令（源码里新增 `Cmd::ServiceDump`），直接在设备上对着
`/dev/binder` 的 handle 0 打各种组合，结果：

| 试验 | 结果 |
|---|---|
| 事务号 1~10 × 4 种头部（strict+workSource+token / strict+token / 仅 token / 空） | **全部** `FAILED_TRANSACTION (-2147483647)` |
| 服务名用 UTF-16 还是 UTF-8 | 无差别，全部失败 |

连空 parcel 都被同样拒绝，说明不是"方法号/编码不对"，而是**这一层 binder 调用在该底座上根本不被目标接受**（handle 0 指向的节点对所有事务都返回失败）。继续往下就得看内核 binder 调试信息（谁是 context manager、handle 表），属于内核/驱动级排查。

→ 判定：**在 TWRP 底座上把 Skyward 的 binder 客户端修通，成本远超收益**。`/data` 解密交给 TWRP。

## 22. 格式化 /data（2026-11-12 13:2x）

- 现象：Skyward 的挂载页里 `data` 不显示文件系统类型，只有 "未挂载 · 494.7GB"，格式化也进不去。
- 实测原因：**该分区当时根本没有文件系统**——读偏移 1024 的 f2fs 魔数是 `00 00 00 00`。
  Skyward 的 `probe::filesystem()` 用 `toybox blkid` 探测，探不到就显示空，属正常行为。
- 处理：用 Skyward 自带的 mkfs 重新建盘（它调 ramdisk 里的 `make_f2fs`）：
  ```bash
  skyward mkfs /dev/block/bootdevice/by-name/userdata f2fs
  ```
  之后 `toybox blkid` 返回 `TYPE="f2fs"`，魔数 `10 20 f5 f2` 正常。
- 说明：这个分区现在是**空的未加密 f2fs**，recovery 里可以直接挂载；但系统一旦开机，
  vold 会按 FBE 重新加密/初始化它。

## 23. v11：未加密 data 直接挂载 + MTP 可见（2026-11-12 13:35）

- 起因：格式化后 Skyward 仍把 data 当"要解密"，挂在 keymint 上死循环；MTP 里"内部存储"指向 `/sdcard`（rootfs，7 GB 空目录）。
- 补丁（两处）：
  1. `crates/skyward-crypto/src/data.rs` 新增 `open_unencrypted()`：不是挂载点时先 `probe::filesystem()`，
     探到 f2fs/ext4 就直接 `mount(..., "rw")`；成功后返回 `Opening`，不碰 keymint；
  2. `crates/skyward/src/decrypt.rs` 的 `Job::Open` 先试 `open_unencrypted()`，失败才走原来的密钥流程；
  3. 同一函数里补建 `/data/media/0` 并 `bind` 到 `/sdcard`，让 MTP／文件管理看到真实容量。
- 结果（实测）：
  - `dmesg`: `skyward: /dev/block/bootdevice/by-name/userdata holds f2fs at /data`
  - `mount`: `/dev/block/sda24 on /data type f2fs (rw,...)`；`/sdcard` 同源，461G/459G 可用
  - Windows MTP：内部存储 460 GB（之前是 7 GB 的 rootfs 假容量）
- 镜像：`out/skyward-m2481-v11.img`，sha256 `4a8fa648…`
- 注意：系统一开机 vold 会重新加密 /data，届时这条快速通道会失效（回落到需要 keymint 的路径）。

## 24. ★ 解密打通（2026-11-12 22:00）—— 四处根因

在 TWRP-16（A16）底座上，Skyward 现在能**完整解密并挂载 /data**：

```
skyward: the check of /dev/block/dm-7 ended with 0
skyward: /dev/block/dm-7 holds f2fs at /data
skyward: the users whose own files are still closed: [0]     ← 要凭据
skyward: the users whose own files are still closed: []      ← 打开完成
mount: /dev/block/dm-7 on /data type f2fs (rw,...,inlinecrypt)
/data/media/0: Alarms Android Audiobooks Customize DCIM Documents ...
```

四处根因（每处都是真问题，改一处不够）：

1. **binder 调用头**（`skyward-binder/src/parcel.rs`）
   libbinder 的调用头是 `strict(0x80000000) + workSource(-1) + 0x53595354`（字节 `54 53 59 53`）。
   Skyward 写的是 `"RECO"`，服务目录的 AIDL 分发认不出接口 → **所有事务号都回 UNKNOWN_TRANSACTION**。
   用 LD_PRELOAD 钩住系统 `service` 二进制的 `BINDER_WRITE_READ` 抓字节对比才定位到。

2. **方法号**（`skyward-binder/src/service.rs`）
   A16 的 `checkService` 是 4（不是 3），`listServices` 是 6。

3. **服务句柄提取**（同文件）
   A16 的 `checkService` 回包把 binder 包在元数据里，Skyward 旧的 `read.object()` 读不到 →
   改成在回包里扫 `flat_binder_object` 的类型字（`85 2a 68 73`）取 handle。

4. **keystore2 的 WAL**（`skyward-crypto/src/keystore2.rs`）
   `persistent.sqlite` 主体只有 4 KB，数据都在 `-wal`（1.4 MB）。原来用 `?immutable=1` 打开会
   **忽略 WAL** → `no such table: blobentry`。改成把 db+wal 复制到 /tmp 再正常打开。

另外两个配套改动：
- `devices/m2481-v14.toml`：keymint 用 root 起（system 起注册不上），带 `ready_service` 等待；
  `[firmware] partition="modem"`、`[partitions] mounted=["persist"]`；
- init rc 补丁（`work/stage15/system/etc/init/keymint-start.rc`）：魅族 vendor rc 的 `on init`
  在 recovery 的 init 里不触发，补 `on late-init` 启动 `vendor.keymint-qti`。

最终镜像：`out/skyward-m2481-final.img`（= v15）。

## 25. 收尾（2026-11-12 22:05）

- 清理了调试残留（`service.rs` 里的 debug_note / 临时辅助函数），重新编译打包并刷机验证：
  `/data` 解密 + 挂载 + 读写全部正常。
- 解密后的分区写入实测：`dd` 64 MB 到 `/sdcard` 正常，`/sdcard` 下是真实用户目录
  （Alarms / Android / DCIM / Documents / Download …）。
- 最终镜像：`out/skyward-m2481-final.img`，sha256 `5811e992d01775ccd1d7cfe322567ed13cad6488f5fa90ca0654ef31594e6e03`
- 补丁集：`port/skyward/patches/`（README + 7 个 .patch），按"必选/可选"分类，
  可直接 `git apply` 或手工合并。

## 26. 电池/温度显示修复（2026-11-12 22:2x）

现象：状态页电流 `0.0A`、温度空。

根因链：
1. `/sys/class/power_supply` 是空的 —— 没有 `battery` 电源；
2. `qti_battery_charger` 的电量计固件跑在 **ADSP** 上，ADSP 没起来；
3. ADSP 起不来的原因是固件找不到：`remoteproc0` 要 `adsp.mdt`，只在 **modem_a**
   分区（/vendor/firmware_mnt）里，而恢复环境默认没挂它；
4. 配 `[firmware] partition="modem_a"` 又会让 Skyward 在 bring-up 阶段直接报
   `the phone's own table does not mount modem_a` 并中止（fstab 里写的是 `modem`，
   by-name 里只有 `modem_a`/`modem_b`）。

修法（配置层，不改代码）：
- 去掉 `[firmware]`；
- 加一个 oneshot 服务：挂 `modem_a` 到 `/vendor/firmware_mnt`，再按固件名
  `adsp.mdt` 找到对应 remoteproc 写 `start`；见 `devices/m2481.toml`。

顺带修 gatekeeper（A16 起注册要过 VINTF 校验）：
- 本机 vendor 只在整份 `manifest_pineapple.xml` 里写了 IGatekeeper，没有 fragment，
  于是 servicemanager 拒绝注册并 abort 进程；
- 往 ramdisk 放一份 fragment `system/etc/vintf/manifest/android.hardware.gatekeeper-service-qti.xml`；
- 效果：gatekeeper 注册成功 → 用户文件解锁（`closed: []`）。

最终状态（实测）：
```
adsp: running        firmware: /vendor/firmware_mnt (modem_a)
battery: current=-583754 µA  temp=290 (29.0℃)  capacity=95  status=Charging
skyward: /dev/block/dm-7 holds f2fs at /data
skyward: the users whose own files are still closed: []     ← 解锁完成
/data/media/0: Alarms Android Audiobooks …
```
镜像：`out/skyward-m2481-final.img`，sha256 `39eebb4e…`；
配置：`devices/m2481.toml`（= v22）。

## 27. WLAN 排查（2026-11-12 22:3x）—— 未解决，卡在 QCA 驱动不出网卡

已修的部分：
- cfg80211 加载失败 `Unknown symbol rfkill_*` —— 本机 `rfkill.ko` 在 **system_dlkm**：
  `/system_dlkm/lib/modules/6.1.25-android14-11-maybe-dirty/kernel/net/rfkill/rfkill.ko`，
  而 Skyward 的 `[modules] sources` 原本没有 system_dlkm，且该分区的模块在版本子目录里。
  处理：把 rfkill.ko 取出，放进 ramdisk 的 `/lib/modules/rfkill.ko`（vendor_boot 源），
  并在 `[wlan].modules` 里排到 qca 驱动前面。
- 结果：`cnss2 / cnss_* / cfg80211 / rfkill / qca_cld3_kiwi_v2` 全部 insmod 成功，
  PCI 上 `0000:01:00.0 (17cb:1107)` 已绑定 `cnss_pci`。

没解决的部分：
- **wlan0 / p2p0 不出现**（等待 20 秒超时）。
- 手工 `rmmod qca_cld3_kiwi_v2` 触发 cnss2 断言
  （`cnss_wlan_unregister_driver+0x190`），写 `/sys/kernel/cnss/fs_ready` 也会把内核打崩重启。
- 这说明恢复环境下只靠"insmod 一串模块"不够：qca 驱动的固件下载走 ADSP/QMI，
  还依赖 vendor wifi HAL（`android.hardware.wifi-service`，带 NET_ADMIN/NET_RAW/SYS_MODULE
  能力）的完整 bring-up 序列；`wlan_mac.bin` 又是指向 persist 的悬空链接。

结论：WLAN 属于"要在恢复环境里重建整套 vendor wifi 栈"的活，比解密那条链更长，
本轮不做。设备保持在可用版本（解密 + 电池/温度正常）。

## 28. WLAN 分阶段日志（2026-11-12 23:0x）

给 `wlan.rs` 加了四段日志（驱动 / supplicant / 地址 / 联网），实测第一段就停住：

```
skyward: wlan 1/4 driver: loading 2 modules for wlan0
skyward: wlan 1/4 driver: modules in after 122 ms, waiting up to 20 s for wlan0
skyward: the WLAN did not come up: the driver made no wlan0 in 20 seconds
kernel : cnss: WLAN register driver deferred for Calibration
```

- 模块加载只用 122 ms，**不是慢，是不通**：qca 驱动注册被"校准"挡住。
- 系统里的触发点是 `on post-fs-data: write /sys/kernel/cnss/fs_ready 1`；
  在恢复环境里写这个节点 → **内核 panic、手机重启**（两次复现，与是否先起 wifi HAL 无关）。
- 第 2/3/4 段（supplicant / DHCP / 联网）因此根本没走到，日志会在驱动阶段就断。

结论：WLAN 的瓶颈被定位在"驱动校准"这一段，且触发校准会让内核崩，需要在恢复环境里补 QMI/IPA 侧的初始化才有解。

## 29. WLAN 第二次尝试（cnss-daemon + 延时校准）—— 仍不通，已回滚

按"先起 cnss-daemon（system），再延时写 fs_ready"做了 v31：

- 延时写（`sleep 8; echo 1 > fs_ready`）**没有再触发内核 panic**（比立刻写安全）；
- 但 `cnss-daemon` 在恢复环境里一启动就退出（`cnss-daemon -n -l` 直接 rc=0、无输出），
  校准因此没人做，驱动一直停在：
  ```
  cnss: WLAN register driver deferred for Calibration
  skyward: wlan 1/4 driver: modules in after 133 ms, waiting up to 20 s for wlan0
  skyward: the WLAN did not come up: the driver made no wlan0 in 20 seconds
  ```
- 副作用：这一版里 ADSP 的 oneshot 没被执行（新增服务改变了启动顺序/失败点），
  电池与温度又变回空的 —— 所以已回滚到 v25。

结论：WLAN 仍卡在"校准"，而校准依赖 cnss-daemon 的 QMI 环境；恢复环境缺这套东西，
继续投入的性价比很低。设备已回到稳定版（解密 + 电池/温度正常）。

## 30. 屏幕锁（PIN）功能（2026-11-13）

目标：给 Skyward 自己加一道开机锁，进恢复界面之前要输 PIN。

实现（改在 UI 框架里）：
- `skyward-ui/src/settings.rs`：`Settings` 增加 `lock: Option<String>`，随 settings.json 持久化；
- `skyward-ui/src/app.rs`：`Screen::Lock` / `Screen::LockSet` 两个页面、`State.lock/lock_stage/lock_first`、
  `Action::LockCheck` / `Action::LockOff`，以及 `lock_entered/lock_chosen/lock_submit`；
- `skyward-ui/src/pages.rs`：`lock()` 与 `lock_set()` 两个页面（复用 `widgets::keypad` / `code_dots` / `button`），
  设置页新增「安全 → 屏幕锁」一行；
- `skyward/src/main.rs`：启动时若 settings 里有 lock，就从 `Screen::Lock` 开屏；另加 `skyward lock [--clear]` 子命令。

行为：
- 设置页点「屏幕锁（未设置）」→ 输 4~8 位 PIN 两遍 → 保存；再点（已开启）→ 关闭；
- 开机先进锁屏页，PIN 正确才进主页，错误会 toast 提示并清空；
- 忘记 PIN 的兜底：目前靠 `skyward lock --clear`（但**在 UI 运行时调用会崩**，见下）或重刷镜像。

已知问题：
- 在 UI 正在运行时执行 `/system/bin/recovery lock --clear` 会让整机掉线（疑似第二次打开显示/DRM 冲突），
  所以这个兜底要改成"开机时按住音量键跳过"或"在 TWRP 里删 settings.json 的 lock 字段"。

## 31. 屏幕锁功能搁置（2026-11-13）

- 功能代码保留（`Screen::Lock/LockSet`、设置页「安全 → 屏幕锁」、`skyward lock [--clear]`），
  但**暂时不启用**：已把 `/metadata/skyward/settings.json` 里的 `lock` 字段删除，
  开机直接进主页。
- 关键修正记录：开机入口是 `fn recovery()`（init 直接跑 `/system/bin/recovery` 无参数），
  锁屏判断必须加在这条路径的 `ui::run(..., Screen::Home, ...)` 处；只加在 `ui` 子命令里不生效。
- 已知坑：UI 运行时执行 `/system/bin/recovery lock --clear` 会让整机掉线（疑似二次打开显示/DRM），
  所以"忘记 PIN"的正确兜底还没做（音量键跳过 / TWRP 删字段）。

## 32. 待办清单（2026-11-13）

WiFi 与屏幕锁的进展/卡点/下一步已单独整理成 [TODO_m2481.md](TODO_m2481.md)，
这里只留一个索引：

- **WiFi**：模块链已通（含 rfkill 从 system_dlkm 取出），卡在 qca 驱动「冷启动校准」；
  `cnss-daemon` 在恢复环境起不来，校准无法完成。下一步是从 Android dump daemon 的启动环境。
- **屏幕锁**：UI 代码已完成（锁屏页、设 PIN 页、设置入口、开机拦截），
  关键修正记录：开机入口是 `fn recovery()` 而不是 `ui` 子命令。
  缺「忘记 PIN」兜底；当前未启用（settings.json 里无 lock 字段）。

## 33. AVB 校验页（2026-11-13）

别人机器上的「AVB 校验」页，本机这版原本没有（上游快照里没有这个页面，但 `avbctl` 引擎是现成的）。
已补上：

- `app.rs`：`Screen::Avb`、`State.avb_flags`、`Effect::ReadAvb` / `Effect::SetAvb(&'static str)`、
  `Action::AvbVerity(bool)` / `AvbVerification(bool)`、`App::set_avb()`；
- `pages.rs`：`avb()` 页面（目标分区 `vbmeta_<slot>`、两个开关：禁用启动验证 / 禁用 dm-verity），
  入口放在「高级」页；
- `ui.rs`：`read_avb()` 直接读 `/dev/block/by-name/vbmeta_<slot>` 头 120 字节处的 flags（大端 u32），
  `Effect::SetAvb` 调 `skyward_install::commands::run("avbctl", …)` 写标志，写完再读一次刷新页面。

标志位含义（vbmeta 头 flags，大端）：
- bit0 = 关闭 dm-verity
- bit1 = 关闭启动验证（verification）

本机当前实测：`vbmeta_a` flags = `0x00000003` → **两项都已关闭**（页面上两个开关都应是"开"）。
镜像：`out/skyward-m2481-v34-avb.img`，sha256 `028d37e8…`

## 34. AVB 页改成 TWRP 形态（2026-11-13）

在 v34 基础上按 TWRP 的做法重做（`out/skyward-m2481-v35-avb.img`，sha256 `9035f8a6…`）：

- 页面上半是信息行：`目标分区 vbmeta_<slot>` + `标志值 0x00000003`
- 中间两个开关只表达"想要的状态"（不再点了就写）
- 只有**和当前 flags 不一致**时才显示「滑动以写入 vbmeta」的滑动条；滑动确认后才调 avbctl
- 底部说明仍然写"只改当前槽位、重启后生效、不解锁 Bootloader"

实现要点：
- `Confirm::Avb` + `confirm_label` 文案；`State.avb_choose` 存"想要的状态"，
  `set_avb()` 读回后同步；`Confirm::Avb` 分支算出要写的命令（verity/verification 各判断一次），
  逐个 `Effect::SetAvb`，最后 `Effect::ReadAvb` 刷新。

另外记一个**重要坑**：`/system/bin/recovery` 这个名字在 `main()` 里是硬编码"直接跑 UI"的
（`if name == "recovery" { return recovery(); }`），所以带参数调用 `recovery avbctl …` 会**再起一个 UI**
把整机搞崩。要跑 CLI 必须用符号名，例如 `/system/bin/avbctl get-verity`。

## 35. AVB 页读写全通（2026-11-13 13:24）

修掉两处拼路径的 bug 后（`Slot::suffix()` 自带下划线，别再补 `_`；`letter()` 是大写），
在真机上完整验证了读与写：

1. 打开「高级 → AVB 校验」：`vbmeta_a`、`0x03`、两个开关都开 ✓
2. 关掉「禁用启动验证」→ 出现「滑动以写入 vbmeta」→ 滑动
3. 页面刷新为 `0x01`、该开关关闭 ✓
4. 命令行复核：
   ```
   /system/bin/avbctl get-verity        → verity is disabled on slot _a
   /system/bin/avbctl get-verification  → verification is enabled on slot _a
   vbmeta_a flags = 00 00 00 01
   ```
   与页面一致，写入生效（重启后由 bootloader 生效）。

最终镜像：`out/skyward-m2481-v36-avb.img`，sha256 `7693fefa…`
（页面实现：`Screen::Avb` + `State.avb_choose` + `Confirm::Avb` 滑动确认 + `Effect::{ReadAvb,SetAvb}`）

## 36. ★ WiFi 突破：wlan0 自动起来了（2026-11-13 21:2x）

**根因**：cnss2 的冷启动校准要从 `/vendor/firmware_mnt/image/kiwi/bdwlan.*` 取固件。
固件分区没挂就写 `fs_ready` → 校准失败 → `force shutdown`（我们之前几次崩溃/掉线就是它）。

**已验证的正确时序**（写进代码，不再依赖 oneshot 服务）：
1. 加载 `rfkill` + `qca_cld3_kiwi_v2`（依赖链由 modules.dep 带出，共 19 个模块）
2. **等驱动稳定 25 秒**（太早写 fs_ready 会让 QMI 在校准后断开、注册不完成）
3. 挂 `modem_a`（vfat ro）到 `/vendor/firmware_mnt`
4. 写 `/sys/kernel/cnss/fs_ready = 1`
5. 校准约 **5.9 秒**（`cnss: Calibration took 5908 ms`）
6. `wlan0` 出现（日志：`wlan0 is up after 44707 ms`）

**代码改动**：
- `skyward-config`：`Wlan` 增加 `calibration = { partition, flag, wait, settle_ms }`
- `skyward/src/wlan.rs` `bring_up()`：在加载模块后按上面的顺序做挂载、等待、写标志、再等校准
- 配置：`devices/m2481-v46.toml`（`[wlan]` 带 `calibration = { partition = "modem_a", flag = "/sys/kernel/cnss/fs_ready", wait = 25, settle_ms = 8000 }`）

**仍卡住的一步**：`wpa_supplicant` 起不来
```
CANNOT LINK EXECUTABLE "/vendor/bin/hw/wpa_supplicant":
cannot locate symbol "sk_dup" referenced by wpa_supplicant
```
- 该 supplicant 是 QTI 的，依赖带 `sk_dup` 的 OpenSSL（libcrypto）；
- 本机 vendor 里**没有** libssl/libcrypto，system 的 BoringSSL 不导出 `sk_dup`；
- 已把 system 的 libcrypto/libssl 打进 ramdisk，仍缺该符号。

**下一步可选**：
1. 找一个导出 `sk_dup` 的 libcrypto（OpenSSL 1.1 系列）放进 ramdisk；
2. 换一个不依赖该符号的 supplicant（本机 system 里没有 AOSP 版）；
3. 停在这里：驱动可用、网卡在，但应用层连接还差 supplicant。

镜像：`out/skyward-m2481-v47-wlan.img`，sha256 `fc194755…`
（电池 28℃/79%、AVB 页、解密等其它功能不受影响；WLAN 开关会等约 45 秒后报 supplicant 错误）

## 37. WiFi 最后一步：supplicant 依赖的 OpenSSL 在本机找不到（2026-11-13 21:4x）

`wpa_supplicant` 缺 `sk_dup` 等符号，逐个查证后结论：

1. 本机 **system 的 libcrypto.so 是 OpenSSL 3**（Meizu 定制，导出 `OPENSSL_sk_*`、
   `OPENSSL_init_crypto`），不是 Android 常规的 BoringSSL；
2. **vendor 分区没有 libcrypto/libssl**（`find` 全盘确认）；
3. conscrypt APEX 里的 libcrypto 与 system 同一份（同样 1569104 字节，OpenSSL 3）；
4. 自己用 NDK 交叉编译了 **OpenSSL 1.1.1w**（`work/libc11.so`），它导出 `OPENSSL_sk_dup`
   但**不导出** supplicant 需要的 `sk_dup`（1.1 起改名为 OPENSSL_ 前缀）；
5. 把 supplicant 的未定义符号与 OpenSSL 1.1 对比，仍缺 **37 个** OpenSSL/BoringSSL 符号，
   其中 `EVP_PKEY_from_keystore`、`X509_Get_Prikey`、`PKCS7_get_certificates`、
   `sk_pop_free_ex` 是 **Android BoringSSL 专有**的，OpenSSL 不会提供。

结论：供应商的 supplicant 是针对 **BoringSSL（Android 13/14 时期的 vendor VNDK 快照）** 编的，
而这台机器（Flyme 12.x + 移植的 vendor）里没有那份库。要补齐只能从外部拿：
- 另一台 m2481 的 stock vendor 里的 `libcrypto.so` / `libssl.so`（BoringSSL 版），或
- 对应的 vendor VNDK 库包（`com.android.vndk.v34` 之类）。

驱动侧已经是好的：wlan0 能自动建出来（见第 36 节）。

## 38. ★ WiFi 通了（2026-11-13 21:45）

WLAN 页能扫到附近网络（CPU-5G / CPU / 4232 / 800 / ChinaNet …），信号和加密标记都正常。

补齐最后两块的经过：

1. **找到正确的 BoringSSL**
   - Android 里 `wpa_supplicant` 实际加载的是
     `/apex/com.android.vndk.v34/lib64/libcrypto.so`（不是 system 的 OpenSSL 3）；
   - 用 KernelSU root 从该路径把 `libcrypto.so`(1.45 MB) / `libssl.so`(414 KB) 抽出来；
   - 它们导出了 supplicant 需要的 `sk_dup`/`sk_*`/`PKCS7_get_certificates`/`sk_pop_free_ex`；
   - `EVP_PKEY_from_keystore` 由 `libkeystore-engine-wifi-hidl.so` 提供（已在镜像里）。

2. **避免覆盖 system 的 libcrypto**（会影响 adbd 等）
   - 把 VNDK 的两个库改名为 `libc11.so` / `libs11.so` 放进 ramdisk；
   - 用 `work/rename_openssl.py` 把 supplicant 的 DT_NEEDED
     `libcrypto.so`→`libc11.so`、`libssl.so`→`libs11.so`（等长字符串原地改，不动其它偏移）；
   - `libs11.so` 自己的 NEEDED 也改成 `libc11.so`；
   - supplicant 本体放到 `/system/bin/wpa_supplicant`（配置 `[wlan].supplicant` 指过去）。

3. **接口刚出现时的 ENODEV 竞态**
   - `wlan0` 的 sysfs 节点先出现、网卡稍后才应答 `SIOCGIFHWADDR`；
   - 在 `wlan.rs` 里把读 MAC 改成最多 5 秒重试，日志 `wlan 2/4 supplicant: answering after 117 ms`。

最终镜像：`out/skyward-m2481-final.img` = `skyward-m2481-v49-wlan.img`，
sha256 `08f17886…`。开 WLAN 后约 55 秒网卡就绪（25s 等驱动 + 8s 校准 + 启动 supplicant）。

### 38.1 四段全部打通（实测日志）

```
wlan 1/4 driver:     wlan0 is up after 44707 ms
wlan 2/4 supplicant: answering after 117 ms
wlan 3/4 address:    wlan0 has 192.168.10.95 (255.255.255.0) through 192.168.10.1 for 86400 s
wlan 4/4 reach:      the network on wlan0 reaches the internet
```

- 驱动：模块 → 等 25 秒 → 挂 modem 固件 → 写 fs_ready → 校准 5.9 秒 → wlan0
- supplicant：VNDK 的 BoringSSL（改名 libc11/libs11）+ 改过 DT_NEEDED 的 wpa_supplicant
- DHCP：拿到 192.168.10.95 / 网关 192.168.10.1
- 联网探测：`connect.rom.miui.com/generate_204` 判定可以上网

至此 WiFi 在 Skyward 恢复环境里完全可用。

## 39. 设备代号改为 meizu21pro（2026-11-13 21:5x）

- `devices/meizu21pro.toml`：`codename = "meizu21pro"`（其余与 v48 配置一致：WLAN 校准 + 改过的 supplicant）
- 开机 banner 实测：`skyward: meizu21pro 0.2.0 recovery up`
- 影响面：主页设备名、终端提示符/HOSTNAME、备份目录命名、在线更新检查（会用 meizu21pro 找 release）
- 镜像：`out/skyward-meizu21pro-final.img`，sha256 `0554a849…`

打包命令相应改为：
```bash
wsl -d MeizuBuild -- bash /mnt/d/MyDrivers/toolchains/packcfg.sh meizu21pro.toml skyward-meizu21pro-<版本>.img
```

## 40. 设备信息与状态栏修正（2026-11-13 22:1x）

**CPU 显示 6.60GHz 的真相**：不是显示算错，是移植内核在开机前几十秒里
**8 个核的 `cpuinfo_max_freq` 全都报 6604800**（真实值 3302400，正好 2 倍），
约一分钟后才变成 2265600/3148800/2956800/3302400。Skyward 在 7 秒时采集，
拿到的就是 6604800，`khz/1e6` = 6.60。

修法：
- `skyward-info` 新增 `cpu_now()`，`pages::specs` **画页面时重新读**，不再用启动时的缓存值；
- `cpu()` 加了个离群保护：最高值若比第二高多 25% 以上就丢掉（防止偶发的单核异常值）。

**电池容量**：内核设计容量 5190mAh，机器宣传是 5050mAh。
- 配置新增 `[device] battery_mah`（本机 5050）；
- UI 三处 `collect()` 之后统一套用 `adopt_rated_capacity()`，页面显示 5050mAh。

**状态栏对齐**：时钟原来按 0.72em 估算基线，和图标（按字形实际居中）算法不一致；
改为 `text_vc`（按字形 ink 垂直居中），与电池数字同一套算法。

镜像：`skyward-meizu21pro-v56.img`，sha256 `05d1b92a…`
（调试日志 `the pages read the phone as ...` 可随时确认采集值）

## 41. 内置浏览器页（2026-11-13 22:5x）

「高级 → 浏览器」：在 recovery 里输入网址、看页面文字与链接、把文件下载到内部存储。

- `crates/skyward/src/browser.rs`：抓取线程。
  - `Task::Fetch`：GET，读 Content-Type/Length/Disposition；HTML 去脚本/样式、剥标签、
    解实体、抽 `<a href>` 并相对路径补全；正文按 42 字折行，最多 400 行、60 个链接。
  - `Task::Download`：流式写文件到 `内部存储/Download`，每 200ms 报进度，可停止（删残文件）。
  - 用 `skyward_net::http::agent`（ureq + rustls，DNS 用 WLAN 拿到的服务器）。
- UI：「高级」新增「浏览器」行（`Icon::Globe`）。
  - 地址栏复用 `widgets::search_field`，点一下弹 `widgets::keyboard`，按键走 `Action::Type(Cap)`
    → `type_key` → `Screen::Browser` 写入 `state.browser.url`；回车即打开。
  - 页面显示状态（HTTP 码或错误）、进度、文件行（名字·大小→「下载到内部存储」）、正文、链接列表
    （点链接继续打开）。
- 限制：不是真浏览器——没有 JS/CSS/图形，只做文字与下载。

镜像：`skyward-meizu21pro-v57.img`，sha256 `28a767ae…`

## 42. 震动（2026-11-13 23:0x）

本机有两路 FF 设备：
- `event3` `qcom-hv-haptics-dis`（PM8550B，FF=120070000）
- `event5` `aac-haptics`（I2C 9-005f，RT6010，`ics_haptic` 驱动）← 系统实际用的马达

配置新增：
```toml
[haptics]
device = "/dev/input/event5"
resonance = ".../i2c-9/9-005f/leds/vibrator_5f/lra_f0"   # 内容形如 "f0 = 1247"（0.1Hz）
strength = { lightest = 0.25, strongest = 1.0 }
```

代码改动：
- `skyward-config`：`Haptics` 增加可选 `device`（不写就仍自动取第一个支持 FF_PERIODIC 的设备）；
- `haptics::motor()` 按配置打开指定设备；
- `haptics::resonance()` 兼容 `f0 = 1245` 这类带标签的写法（取最后一个纯数字）。

实测（设备内置探针）：
```
/tmp/skyward haptic --config /skyward/device.toml --position 1
haptics /dev/input/event5: clicks and ticks made for a motor swinging at 124.4 Hz
played the click at 1.000 of the full strength, 26 ms long
```
镜像：`skyward-meizu21pro-v58.img`，sha256 `66a93036…`

### 42.1 修「炸 UI」：马达文件不在固定路径（2026-11-13 23:2x）

现象：刷了带点击震动的版本后 UI 黑屏、进程反复重启。日志：

```
skyward: recovery failed: read /sys/.../i2c-9/9-005f/leds/vibrator_5f/lra_f0: No such file or directory
```

原因两层：
1. `Haptics::open` 在 UI 启动时失败 → `ui::run` 返回错误 → init 重启 recovery → 死循环；
2. 更深一层：haptics 芯片的 **i2c 总线号每次开机不一样**（一次是 `i2c-9`，另一次是 `i2c-8`），
   写死的 sysfs 路径必然失效。

修法：
- UI 启动时打不开马达只记一条 kmsg，不再让 UI 挂掉；第一次点击时再尝试打开（懒加载）。
- 马达设备按**驱动名** `aac-haptics` 认（event 号也会变），也接受绝对路径。
- 谐振频率文件找不到时，在 `/sys/devices` 下递归找 `vibrator*/lra_f0`（或 `f0_cali`）。

镜像：`skyward-meizu21pro-v62.img`，sha256 `8d0ba85a…`

## 43. 升级到 Skyward 0.3.1 + 震动根因（2026-11-15）

**升级**：官方 v0.3.1（大重构：UI 拆成 app/*.rs、新增 omapi/programs 等）。
以 0.3.1 为基线重新移植了：binder 头标 TSYS、服务目录编号/handle 扫描、
keystore2 WAL、未加密 data 直挂、packer `--extra/--allow-custom`、
WLAN 冷启动校准、魅族面板背光回退、UFS 机型配置。
镜像：`out/skyward-meizu21pro-v035.img`。

**震动根因**：AAC（RT6010）马达驱动在开机 ~0.96s probe，早于 Skyward 挂载
`/vendor`（1.42s）和 `/persist`（1.52s），读不到芯片配置和 f0 标定
（`get slot cali data:-19`），LED/FF 节点建不出来，功放不工作。

修复尝试：把 `haptic_aac.ko` 从 ramdisk 的 `lib/modules/modules.load.recovery`
里去掉（注意必须 LF 换行，CRLF 会让 init 一个模块都加载不了 → 黑屏），
改由 Skyward 在挂载完成后 `[modules].load = ["goodix_ts", "haptic_aac"]` 加载。
v035 实测驱动 probe 落在 1.44s：
```
vibrator_init: led cdev framework!
freq data:1300 / get slot cali data:1300 / set_f0 = 1300   ← 标定成功
```
但 **FF 和 LED 两种播放方式都没有实际震动**：
- `EVIOCSFF` 上传回 Ok，`play` 执行，硬件不响；
- LED `duration/activate`、`play` 都被接受，`state` 复位，仍不响；
- RichTap `libaacvibrator.so` 的 `aac_vibra_init` 返回 -1；
- 驱动仍报 `loading /vendor/firmware_mnt/image/haptic_config.bin failed -13`，
  这个 414 字节的芯片配置没有真正加载，怀疑功放增益/输出被关。

后续可选：找 `haptic_config.bin` 的真实来源（/vendor/etc/Hapticsconfig.xml、
/vendor/etc/acdbdata/haptics_data），或逆 RichTap 的 init 条件。

## 44. ★ 震动修好（2026-11-15 13:3x）

**最终方案：绕开 evdev FF，改用驱动的 LED 接口。**

```sh
/sys/class/leds/vibrator_5f/duration   # 写毫秒
/sys/class/leds/vibrator_5f/activate   # 写 1 开始
```

代码：
- `skyward-config`：`Haptics` 新增 `led: Option<String>`；
- `haptics.rs`：`Haptics` 拆成 `Motor::Force`（原 FF 路线）/`Motor::Led`（新路线）；
  - click：`duration = 22 + share*26` 毫秒，再 `activate=1`；
  - ticks：45ms → 停 55ms → 30ms；
- 配置：`[haptics] led = "/sys/class/leds/vibrator_5f"`。

**另一个前提（关键）**：`haptic_aac.ko` 不能由 init 在开机 0.9s 加载，否则 probe 早于
`/vendor`、`/persist` 挂载，f0 标定失败（-19）功放不工作。做法是把该模块从
ramdisk 的 `lib/modules/modules.load.recovery` 里删掉（文本必须 LF 换行），
改由 Skyward 在挂载后用 `[modules].load = ["goodix_ts", "haptic_aac"]` 加载。
v036 实测 probe 在 1.44s，`freq data:1300`、`set_f0 = 1300`。

**验证**：CLI 探针（1 满强度 + 3 中等 + 1 滴答）与界面点击（设置项/开关/返回箭头）
均有震动。

最终镜像：`out/skyward-meizu21pro-final.img`
（= v036，sha256 `b57c4c77…`），备用 `out/skyward-meizu21pro-v032.img`。
