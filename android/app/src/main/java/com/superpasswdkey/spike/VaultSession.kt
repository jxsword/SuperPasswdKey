package com.superpasswdkey.spike

import android.os.Handler
import android.os.Looper
import android.util.Log
import java.io.File
import java.util.concurrent.Executors
import uniffi.sp_kffi.FfiKdfParams
import uniffi.sp_kffi.Vault

/**
 * SP-F-802 解锁会话（design_docs/14 §4）：进程内单例，持 kffi Vault 句柄。
 * - 空闲超时（默认 300s，对齐 SP-F-703）自动调 vault.lock()（actor drop，凭据随库零化）
 * - 懒检查兜底：任何访问前若已超时则先锁库
 * - 凭据只在 unlock/createVault 调用栈内存在，不落任何持久层（SP-F-804）
 * - 依赖纪律：仅用标准库执行器（壳层不引入 coroutine 依赖）
 * - 测试注入点（SP-F-802 验收"可注入时钟"）：[idleTimeoutMs] 超时时长与 [nowMs]
 *   时钟均可替换；e2e 经 TestActivity 的 idle_timeout_ms 缩短超时做秒级验证
 */
object VaultSession {
    private const val TAG = "SPIKE"

    /** 壳层库文件名（应用私有 filesDir 下）——库路径单一来源 */
    const val VAULT_NAME = "vault.kdbx"

    /** DR-010/011 终态：移动端校准目标 ~1s（KeePassXC 口径，≤2s 体验线） */
    val CALIBRATION_TARGET_MS: UInt = 1000u

    const val DEFAULT_IDLE_TIMEOUT_MS = 300_000L

    /** 空闲锁库超时（可注入：e2e 缩短至秒级验证超时锁库路径） */
    @Volatile
    var idleTimeoutMs: Long = DEFAULT_IDLE_TIMEOUT_MS

    /** 时钟注入点（默认系统墙上时间） */
    @Volatile
    internal var nowMs: () -> Long = System::currentTimeMillis

    private var vault: Vault? = null
    private var lastActiveMs = 0L
    private val main = Handler(Looper.getMainLooper())
    private val io = Executors.newSingleThreadExecutor { r -> Thread(r, "sp-session") }
    private val lockRunnable = Runnable { lockNow() }

    /** 壳内库文件的规范路径（filesDir 下 [VAULT_NAME]） */
    fun vaultPath(filesDir: File): File = File(filesDir, VAULT_NAME)

    val isUnlocked: Boolean
        get() = synchronized(this) { vault != null && !idleExpired() }

    /** 读取 vault 前的懒超时检查；未解锁返回 null（锁后机密操作即拒绝） */
    fun activeVault(): Vault? = synchronized(this) {
        if (idleExpired()) lockNowInternal()
        vault
    }

    private fun idleExpired(): Boolean =
        vault != null && nowMs() - lastActiveMs > idleTimeoutMs

    /**
     * 解锁（KDF 阻塞数秒，调用方须在后台线程执行）。
     * 旧会话先锁库；失败抛 FfiException，由调用方呈现。凭据仅存于本次调用栈。
     */
    fun unlock(dbPath: String, password: String, keyfilePath: String?): Vault =
        synchronized(this) {
            vault?.let { runCatching { it.lock() } }
            val v = uniffi.sp_kffi.openDb(dbPath, password, keyfilePath)
            vault = v
            lastActiveMs = nowMs()
            scheduleIdleLock()
            Log.i(TAG, "session unlocked (${File(dbPath).name})")
            v
        }

    /**
     * 首次初始化（DR-010/011 终态）：calibrate_kdf 实测校准（m=64MiB/p=4 固定，
     * t∈[2,8] 校准至 ~1s；阻塞数秒，调用方须在后台线程执行并先行 UI 提示）
     * → 以校准参数建库（Argon2 参数随 KDBX4 头存储，无需另行持久化）→ 解锁。
     * 校准/建库失败抛 FfiException 由调用方呈现，不做静默兜底
     * （FfiKdfParams::default 的"跳过校准"兜底仅供 fixture/测试口径）。
     */
    fun createVault(dbPath: String, password: String, keyfilePath: String?): Pair<Vault, FfiKdfParams> =
        synchronized(this) {
            val params = uniffi.sp_kffi.calibrateKdf(CALIBRATION_TARGET_MS)
            uniffi.sp_kffi.createDb(dbPath, password, keyfilePath, params)
            Log.i(
                TAG,
                "vault created (calibrated m=${params.memoryKib} t=${params.iterations} p=${params.parallelism})",
            )
            val v = unlock(dbPath, password, keyfilePath)
            v to params
        }

    fun touch() {
        synchronized(this) {
            if (vault != null) {
                lastActiveMs = nowMs()
                scheduleIdleLock()
            }
        }
    }

    /** 显式锁库（SP-F-802）：drop 库句柄并取消定时器 */
    fun lockNow() {
        synchronized(this) {
            lockNowInternal()
        }
    }

    private fun lockNowInternal() {
        val v = vault ?: return
        vault = null
        main.removeCallbacks(lockRunnable)
        io.execute { runCatching { v.lock() } }
        Log.i(TAG, "session locked (idle timeout or explicit)")
    }

    private fun scheduleIdleLock() {
        main.removeCallbacks(lockRunnable)
        main.postDelayed(lockRunnable, idleTimeoutMs)
    }
}
