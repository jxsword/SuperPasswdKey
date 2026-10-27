package com.superpasswdkey.spike

import android.os.Handler
import android.os.Looper
import android.util.Log
import java.io.File
import java.util.concurrent.Executors
import uniffi.sp_kffi.Vault

/**
 * SP-F-802 解锁会话（design_docs/14 §4）：进程内单例，持 kffi Vault 句柄。
 * - 空闲超时（默认 300s，对齐 SP-F-703）自动调 vault.lock()（actor drop，凭据随库零化）
 * - 懒检查兜底：任何访问前若已超时则先锁库
 * - 凭据只在 unlock 调用栈内存在，不落任何持久层（SP-F-804）
 * - 依赖纪律：仅用标准库执行器（壳层不引入 coroutine 依赖）
 */
object VaultSession {
    private const val TAG = "SPIKE"
    const val IDLE_TIMEOUT_MS = 300_000L

    private var vault: Vault? = null
    private var lastActiveMs = 0L
    private val main = Handler(Looper.getMainLooper())
    private val io = Executors.newSingleThreadExecutor { r -> Thread(r, "sp-session") }
    private val lockRunnable = Runnable { lockNow() }

    val isUnlocked: Boolean
        get() = synchronized(this) { vault != null && !idleExpired() }

    /** 读取 vault 前的懒超时检查；未解锁返回 null */
    fun activeVault(): Vault? = synchronized(this) {
        if (idleExpired()) lockNowInternal()
        vault
    }

    private fun idleExpired(): Boolean =
        vault != null && System.currentTimeMillis() - lastActiveMs > IDLE_TIMEOUT_MS

    /**
     * 解锁（KDF 阻塞数秒，调用方须在后台线程执行）。
     * 旧会话先锁库；失败抛 FfiException，由调用方呈现。凭据仅存于本次调用栈。
     */
    fun unlock(dbPath: String, password: String, keyfilePath: String?): Vault =
        synchronized(this) {
            vault?.let { runCatching { it.lock() } }
            val v = uniffi.sp_kffi.openDb(dbPath, password, keyfilePath)
            vault = v
            lastActiveMs = System.currentTimeMillis()
            scheduleIdleLock()
            Log.i(TAG, "session unlocked (${File(dbPath).name})")
            v
        }

    fun touch() {
        synchronized(this) {
            if (vault != null) {
                lastActiveMs = System.currentTimeMillis()
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
        main.postDelayed(lockRunnable, IDLE_TIMEOUT_MS)
    }
}
