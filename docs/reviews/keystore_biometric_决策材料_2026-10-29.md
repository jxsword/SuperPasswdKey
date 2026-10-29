# Keystore/生物识别解锁 决策材料（DR 草案前置，步骤 6 任务 3）

- 日期：2026-10-29
- 性质：**仅决策材料——不拍板、不实现**；真正拍板由用户另行决策并走 DR 流程
- 联动：与 DR-012（Autofill 保存凭据来源=A1 actor 内保留因子）同触"凭据驻留"边界，本材料将其作为基线一并论证
- 关联：design_docs/14 §4（v1.2 驻留窗口声明）、SP-F-802/804/805、红线 #4#5#7

## 1. 问题定义

当前模型（M2 终态）：解锁会话纯内存——凭据因子随 open_db 留驻 sp-kffi actor（DR-012 A1），
300s 空闲/显式锁库即 drop 清零；**进程被杀即失锁，无持久化恢复**。

待决策问题（两个子问题，宜一次论证）：
1. **会话缓存**：进程被杀后是否允许"免主密码恢复解锁态"（Keystore 包裹凭据/派生密钥落盘）？
2. **生物识别**：BiometricPrompt 是解锁的**附加因子**（主密码仍可用）还是**替代因子**（日常免主密码）？

## 2. 威胁模型（对齐 design_docs/09§4 诚实边界原则）

| 对手 | 当前模型（纯内存） | Keystore 缓存模型 | 备注 |
|---|---|---|---|
| 本地内存注入者（root/越权进程） | 已越权即全失（A1 论证：不降低等级） | 同左；Keystore 硬件背书可挡"非 root 读盘"，挡不住 root+Keystore 攻击面复杂化 | 红线 #5 口径：可确定性清零 |
| 物理接触者（锁屏前/后） | 进程存活期内可解锁；进程死后须主密码 | **可尝试生物伪造/设备凭据**——硬件背书密钥+生物门禁是唯一防线；`setInvalidatedByBiometricEnrollment` 防新增指纹攻击 | 主要收益与主要新增风险同源 |
| 盘上攻击（backup/提取文件） | vault.kdbx 本身 KDBX4 加密；会话零落盘 | **新增落盘物**：Keystore 包裹的凭据密文（含包裹密钥的硬件 TEE 保护） | `allowBackup=false` 已关；密文强度依赖 Keystore 实现 |
| 生物数据 | 不接触 | 系统托管，App 不存生物特征（仅结果/加密门禁） | BiometricPrompt 系统侧负责 |

关键边界：KDBX4 的加密根是主密码派生钥（+keyfile）。**任何"免主密码解锁"方案的本质都是把
等价于主密码（或派生钥）的秘密以 Keystore 包裹形式落盘**——这是与红线 #5（机密不落盘、可清零）
直接冲突、必须由用户明示豁免的决策点。DR-012 A1 的内存驻留不落盘，与此有本质区别。

## 3. 行业对标（事实核实，2026-10-29）

| 产品 | 方案 | 存储 | 因子关系 | 来源 |
|---|---|---|---|---|
| Bitwarden (Android) | 生物识别解锁（可选开启） | 启用后**加密密钥落盘**，由 Android Keystore（硬件背书）+ 生物绑定包裹 | 生物=**解锁替代**（登录仍须主密码+2FA；日常打开用生物/PIN）；主密码变更后生物解锁失效待重验 | [官方文档](https://bitwarden.com/help/biometrics)、[密钥落盘分析](https://www.reddit.com/r/Bitwarden/comments/1eyrdo9/psa_bitwarden_mobile_stores_encryption_keys_on)、[主密码变更失效](https://github.com/bitwarden/mobile/issues/1871) |
| KeePassDX | Device unlocking（可选开启，库级链接） | 库凭据密文落盘，Keystore 密钥包裹；生物变更/Keystore 失效须重新链接 | 生物/设备凭据=**解锁替代**（快解锁）；首次链接须主密码 | [GitLab](https://gitlab.com/kunzisoft/KeePassDX)、[失效处理](https://github.com/Kunzisoft/KeePassDX/issues/1781)、[设置项](https://github.com/Kunzisoft/KeePassDX/issues/2535) |
| Android 官方姿势 | BiometricPrompt + CryptoObject | 密钥在 Keystore/TEE；`setUserAuthenticationRequired` + 每操作鉴权或时间窗 | **CryptoObject 加密绑定**为安全推荐；时间窗模式（validityDurationSeconds）有已 documented 的绕过弱化 | [官方指南](https://medium.com/androiddevelopers/biometric-authentication-on-android-part-1-264523bce85d)、[时间窗弱化](https://github.com/android/security-samples/issues/20)、[invalidation 机制](https://medium.com/mutualmobile/biometric-authentication-with-cryptography-in-android-mutual-mobile-be8f29578192) |

对标结论：两家头部产品都是"**可选开启 + Keystore 包裹落盘 + 生物作解锁替代**"；主密码始终保留为
兜底因子。Aegis 的失效处理 bug（[参考](https://github.com/beemdevelopment/Aegis/issues/332)）说明
invalidation 边界是易错高发区。

## 4. 选项矩阵

| 选项 | 内容 | 安全 | 体验 | 工程代价 | 红线冲突 |
|---|---|---|---|---|---|
| **S0（现状）** | 纯内存会话，进程死即锁 | 基线（不落盘） | 进程被杀须重输主密码 | — | 无 |
| **S1 Keystore 会话缓存** | 解锁时把凭据因子（或等价派生钥）以 Keystore 密钥包裹落盘；进程重启后生物/设备凭据门禁解锁 | 物理接触者攻击面**新增**（生物伪造/设备凭据=唯一防线）；盘上多一件密文 | 进程被杀免重输；对齐行业惯例 | 中：Keystore 包裹 + 生命周期（主密码变更/生物变更失效）+ sp-kffi 接口 + 设置 UI | **红线 #5 需用户明示豁免**（等价秘密落盘，虽为密文且 TEE 保护） |
| **B1 生物=附加因子** | 解锁须主密码 AND 生物 | 强于现状但反 UX（双重摩擦） | 差——每次解锁两步 | 低 | 无 |
| **B2 生物=解锁替代（可选开启）** | 开启后日常用生物解锁（Keystore 包裹，同 S1）；主密码保留兜底 | 同 S1 | 对齐 Bitwarden/KeePassDX 惯例 | 同 S1（与 S1 天然同一实现面） | 同 S1 |

要点：S1 与 B2 是**同一实现面**（Keystore 包裹落盘 + 生物门禁取回）——真正要拍板的是
"是否允许落盘"这一个红线豁免决策；豁免后附加因子（B1）无独立价值。

## 5. 推荐倾向（非拍板）

**倾向：S1+B2 合并方案，但以"默认关闭的实验开关"交付**——

- 安全论证延续 DR-012 A1 同款逻辑：TEE/StrongBox 硬件背书下，盘上密文不降低"注入者已越权"
  场景的等级；物理接触场景由"生物伪造 vs 主密码猜测"权衡，行业（Bitwarden/KeePassDX）均接受
  并默认关闭。
- 红线 #5 豁免必须显式：默认关闭 + 开启时明示"等价秘密将以 Keystore 包裹落盘"确认框；
  关闭开关即销毁落盘包裹（可清零语义恢复）。
- 生命周期硬要求（照抄行业教训）：主密码变更/生物变更 → 包裹失效回退主密码；CryptoObject
  加密绑定（不使用时间窗模式）；`setInvalidatedByBiometricEnrollment(true)`。
- sp-kffi 接口影响预估：新增"以外部提供的主密钥材料开库"入口（Keystore 解包裹后喂给
  open_db 等价路径），不触碰 sp-core/sp-kdbx 公开 API——与 DR-006 桌面适配器预留点同构。

**不推荐**：B1（附加因子）——纯 UX 损害无安全增益；无 Keystore 的时间窗缓存——弱化绑定有
documented 绕过面。

## 6. 待用户拍板的问题清单

1. 是否接受红线 #5 的定向豁免（Keystore 包裹落盘，默认关闭+明示确认）？
2. 若接受：生物识别定位=解锁替代（B2）；主密码兜底保留是否确认？
3. 交付节奏：M3 主线 or backlog？与桌面 DR-006 会话缓存的联动论证是否合并一个 DR？
4. 真机前置：StrongBox/TEE 设备矩阵（最低支持口径）是否先于实现明确？

## 7. 来源清单

- Bitwarden 官方：https://bitwarden.com/help/biometrics （实测可达）
- Bitwarden 密钥落盘分析：https://www.reddit.com/r/Bitwarden/comments/1eyrdo9/psa_bitwarden_mobile_stores_encryption_keys_on
- Bitwarden 主密码变更失效：https://github.com/bitwarden/mobile/issues/1871
- KeePassDX 仓库：https://gitlab.com/kunzisoft/KeePassDX
- KeePassDX 失效处理：https://github.com/Kunzisoft/KeePassDX/issues/1781
- KeePassDX 设置项：https://github.com/Kunzisoft/KeePassDX/issues/2535
- Android 官方 Biometric 指南：https://medium.com/androiddevelopers/biometric-authentication-on-android-part-1-264523bce85d
- 时间窗弱化（官方 samples issue）：https://github.com/android/security-samples/issues/20
- Invalidation 机制：https://medium.com/mutualmobile/biometric-authentication-with-cryptography-in-android-mutual-mobile-be8f29578192
- Aegis 失效 bug 参考：https://github.com/beemdevelopment/Aegis/issues/332
