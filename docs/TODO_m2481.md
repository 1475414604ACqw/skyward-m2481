# m2481（MEIZU 21 Pro）Skyward 待办与已有进展

> 记账文件：这里只记「没做完的事」和「已经走到哪一步」，完整过程见 [BUILD_m2481.md](BUILD_m2481.md)。
> 当前稳定镜像：`out/skyward-m2481-final.img`（sha256 `39eebb4e…`），解密 / 电池 / 温度 / MTP / 刷写 / 备份 均正常。

---

## 1. WiFi（暂停）

### 目标
在 Skyward 恢复环境里连上 WiFi，用于在线更新、联网备份等。

### 已经做完
| 项 | 结果 |
|---|---|
| 模块依赖 | `cnss2 / cnss_* / cfg80211 / rfkill / qca_cld3_kiwi_v2` 全部能 insmod |
| `rfkill` 缺失 | 已解决：本机 `rfkill.ko` 在 **system_dlkm** 的版本子目录<br>`/system_dlkm/lib/modules/6.1.25-android14-11-maybe-dirty/kernel/net/rfkill/rfkill.ko`<br>取出后放进 ramdisk 的 `/lib/modules/rfkill.ko`，排在 qca 驱动之前 |
| PCIe | `0000:01:00.0 (17cb:1107)` 已绑定 `cnss_pci` |
| 固件 | `modem_a` → `/vendor/firmware_mnt` 已能挂上，`adsp.mdt` 可读 |
| 分阶段日志 | `wlan.rs` 已加 `wlan 1/4 … 4/4` 四段日志，可直接看出卡在哪段 |

### 卡在哪
```
skyward: wlan 1/4 driver: loading 2 modules for wlan0
skyward: wlan 1/4 driver: modules in after 133 ms, waiting up to 20 s for wlan0
kernel : cnss: WLAN register driver deferred for Calibration
skyward: the WLAN did not come up: the driver made no wlan0 in 20 seconds
```
- 驱动**加载成功但注册被「校准」挂起**，网卡不出现；第 2/3/4 段（supplicant / DHCP / 联网）根本走不到。
- 系统里的校准触发点是 `on post-fs-data: write /sys/kernel/cnss/fs_ready 1`。
  - **立刻写**：内核断言 panic、整机重启（复现 2 次）。
  - **延时写**（先起 `cnss-daemon`、sleep 8 再写）：不崩了，但仍不生效。
- `cnss-daemon` 在恢复环境里**一启动就退出**（`-n -l` 直接 rc=0、无输出），而 Android 里它以 `system` 身份常驻；
  说明它依赖 Android 启动过程中才齐备的 QMI/qrtr/IPA 环境。

### 下一步（按性价比排序）
1. 从运行中的 Android 里 dump `cnss-daemon` 的启动环境（fd、socket、props、依赖的 qrtr 节点），在恢复环境里复刻。
2. 查 `cnss` 的 "deferred for Calibration" 是否还有别的触发方式（如 `cnss2` 的 debugfs / `recovery` 节点）。
3. 如果都不行，就放弃：WLAN 需要重建半个网络栈，性价比低。
   - 替代方案：MTP 传文件（已可用）；要 WiFi 就进系统或 TWRP。

### 相关镜像（未启用，仅留档）
- `out/skyward-m2481-v26.img` … `v31.img`：逐步尝试的版本
- `devices/m2481-v26.toml` … `v31.toml`：对应配置（`[wlan]`、`cnss-daemon` 服务、延时校准等）

---

## 2. 屏幕锁（暂停）

### 目标
给 Skyward 自己加一道开机锁：进恢复界面前必须输 PIN。

### 已经做完（代码已实现并编译通过）
| 文件 | 改动 |
|---|---|
| `crates/skyward-ui/src/settings.rs` | `Settings` 增加 `lock: Option<String>`，随 `settings.json` 持久化 |
| `crates/skyward-ui/src/app.rs` | `Screen::Lock` / `Screen::LockSet`；`State.lock/lock_stage/lock_first`；`Action::LockCheck` / `Action::LockOff`；`lock_entered` / `lock_chosen` / `lock_submit` |
| `crates/skyward-ui/src/pages.rs` | `lock()` 与 `lock_set()` 两个页面（复用 `widgets::keypad` / `code_dots` / `button`）；设置页「安全 → 屏幕锁」一行 |
| `crates/skyward/src/main.rs` | 开机入口 `fn recovery()` 里按 settings 决定起始页（`Screen::Lock` / `Screen::Home`）；另加 `skyward lock [--clear]` |

### 行为（设计）
- 设置 → 安全 → **屏幕锁**：未设置时点开 → 输 4~8 位 PIN 两遍；已开启时点一下 → 关闭
- 开机先进锁屏页，PIN 正确才进主页；错误 toast「PIN 不正确」并清空
- PIN 存在 `/metadata/skyward/settings.json` 的 `lock` 字段（明文，`/metadata` 只有 root 可读）

### 关键坑（已踩）
- **开机入口不是 `ui` 子命令**：init 直接跑 `/system/bin/recovery`（无参数）→ `fn recovery()`。
  锁屏判断必须加在这条路径的 `ui::run(..., Screen::Home, ...)` 处；只加在 `Cmd::Ui` 里不生效（第一版就是这么翻车的）。
- **不要在 UI 运行时执行 `recovery lock --clear`**：会让整机掉线（疑似二次打开显示/DRM 冲突）。

### 下一步
1. 补「忘记 PIN」的正经兜底（二选一）：
   - 开机时按住音量下键跳过（需要读 `/dev/input/event*` 的 `KEY_VOLUMEDOWN`）
   - 或文档化：进 TWRP 删 `/metadata/skyward/settings.json` 里的 `lock` 字段
2. 把 PIN 改成加盐哈希存储（现在是明文）。
3. 实机验证一遍完整流程（设 PIN → 重启 → 输对/输错 → 设置里关闭）。

### 相关镜像（未启用，仅留档）
- `out/skyward-m2481-v32-lock.img`（开机入口判断写死在 Home，不生效）
- `out/skyward-m2481-v33-lock.img`（入口判断已修好；settings 里没有 `lock` 时不生效）

### 快速启用/停用（不用重编译）
- 启用：在 `/metadata/skyward/settings.json` 的 `"file_transfer": true,` 后面加一行 `"lock": "你的PIN",`
  （需要 `mount -o remount,rw /metadata`，改完 `sync`）
- 停用：删掉这一行（本次就是这么停用的）

---

## 3. 其他备忘
- 固件版本：系统 Flyme 12.7.0.0A（Android 16），恢复底座用 TWRP-16 的 ramdisk。
- 稳定镜像的完整补丁集在 [`patches/README.md`](patches/README.md)。
- 每次改完源码的打包命令：
  ```bash
  wsl -d MeizuBuild -- bash /mnt/d/MyDrivers/toolchains/packcfg.sh m2481.toml skyward-m2481-vXX.img
  ```
  然后 `fastboot flash recovery_a/recovery_b` 两槽。

---

## WiFi：已完成（2026-11-13）

不再属于待办。要点：驱动校准需要先挂 modem 固件；supplicant 需要 VNDK 的 BoringSSL
（`libc11.so`/`libs11.so` + 改过 DT_NEEDED 的 supplicant）。详见 BUILD_m2481.md 第 36~38 节。
镜像：`out/skyward-m2481-final.img`（sha256 `08f17886…`）。
