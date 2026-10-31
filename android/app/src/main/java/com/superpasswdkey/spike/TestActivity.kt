package com.superpasswdkey.spike

import android.app.Activity
import android.graphics.Typeface
import android.os.Bundle
import android.os.SystemClock
import android.util.Log
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import uniffi.sp_kffi.FfiException
import uniffi.sp_kffi.openDb
import java.io.File

/**
 * e2e harness（自 MainActivity 分离，M1 正式化任务 2：演示/测试代码与产品壳入口分离）。
 * 断言集沿用 spike 口径（SP-F-801 模拟器 e2e）+ M1 增量：
 *  - 首建校准链路（DR-010/011 终态）：extra `init_vault=true` 时经 VaultSession.createVault
 *    走 calibrate_kdf→create_db→unlock，断言校准参数落在 DR-010 边界内；
 *  - 空闲锁库注入测试（SP-F-802 验收"可注入时钟"）：extra `idle_timeout_ms` 缩短超时，
 *    断言超时后会话锁定且机密操作拒绝（activeVault()==null）。
 * 运行前置：
 *   1. cargo ndk -t x86_64-linux-android --platform 28 -- build -p sp-kffi
 *      并复制 target/x86_64-linux-android/debug/libsp_kffi.so 到 app/src/main/jniLibs/x86_64/
 *   2. 生成 fixture（口令经环境变量，不落源码）：
 *      SP_TEST_DB_PASSWORD=... SP_TEST_ENTRY_PASSWORD=... \
 *        cargo run -p sp-kffi --example make_spike_vault -- test_vault.kdbx
 *   3. 推送 fixture（run-as 写入应用私有目录，即壳层库文件 vault.kdbx）：
 *      MSYS_NO_PATHCONV=1 adb push test_vault.kdbx /data/local/tmp/
 *      adb shell run-as com.superpasswdkey.spike cp /data/local/tmp/test_vault.kdbx files/vault.kdbx
 *   4. 启动（凭据经 intent extra 传入，不落 SharedPreferences/日志；exported=false，
 *      adb shell 经 START_ANY_ACTIVITY 仍可直接启动）：
 *      MSYS_NO_PATHCONV=1 adb shell am start -n com.superpasswdkey.spike/.TestActivity \
 *        --es db_password ... --es entry_password ... --es entry_username demo-user \
 *        [--ez init_vault true] [--ei idle_timeout_ms 2000]
 * 结果：logcat -s SPIKE 或 run-as cat files/result.txt
 */
class TestActivity : Activity() {
    private val lines = StringBuilder()

    private fun line(s: String) {
        lines.appendLine(s)
        Log.i(TAG, s)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val tv = TextView(this).apply {
            setPadding(32, 32, 32, 32)
            textSize = 13f
            typeface = Typeface.MONOSPACE
        }
        setContentView(ScrollView(this).apply { addView(LinearLayout(this@TestActivity).apply { orientation = LinearLayout.VERTICAL; addView(tv) }) })
        try {
            run()
        } catch (t: Throwable) {
            line("FATAL: ${t::class.simpleName}: ${t.message}")
        }
        tv.text = lines.toString()
        try {
            File(filesDir, "result.txt").writeText(lines.toString())
        } catch (_: Exception) {
        }
    }

    private fun vmHwmKb(): Long =
        File("/proc/self/status").readLines()
            .firstOrNull { it.startsWith("VmHWM:") }
            ?.trim()?.split(Regex("\\s+"))?.get(1)?.toLong() ?: -1

    private fun run() {
        line("== SuperPasswdKey M1 e2e ==")

        val dbPassword = intent.getStringExtra("db_password")
        val entryPassword = intent.getStringExtra("entry_password")
        val entryUsername = intent.getStringExtra("entry_username") ?: "demo-user"
        val entryUrl = intent.getStringExtra("entry_url") ?: "http://127.0.0.1:8080"
        val initVault = intent.getBooleanExtra("init_vault", false)
        val idleTimeoutMs = intent.getIntExtra("idle_timeout_ms", -1)
        if (dbPassword == null || entryPassword == null) {
            line("FAIL: missing intent extras db_password/entry_password")
            return
        }
        // 空闲锁库注入测试前置：缩短超时须在任何解锁前设置（SP-F-802 可注入点）
        if (idleTimeoutMs > 0) VaultSession.idleTimeoutMs = idleTimeoutMs.toLong()

        val fixture = VaultSession.vaultPath(filesDir)
        if (!fixture.exists()) {
            line("PENDING: fixture missing — push test_vault.kdbx via run-as (see TestActivity doc)")
            return
        }
        line("fixture: ${fixture.path} (${fixture.length()} bytes)")

        // --- ① open_db（KDF 64MiB Argon2id 在本次调用内完成）---
        val hwmBefore = vmHwmKb()
        val t0 = SystemClock.elapsedRealtime()
        val vault = openDb(fixture.absolutePath, dbPassword, null)
        val openMs1 = SystemClock.elapsedRealtime() - t0
        val hwmAfter = vmHwmKb()
        line("PASS: openDb #1 (KDF 64MiB Argon2id t=8 p=4) ${openMs1} ms; " +
            "VmHWM ${hwmBefore}->${hwmAfter} kB (delta ${hwmAfter - hwmBefore} kB) " +
            "[非真机: x86_64 emulator Pixel_7_API34]")

        // --- ② find_entry 全字段读回（对照 crates/sp-kdbx/tests 语义）---
        val e = vault.findEntry("Demo Login")
        check(e.username == entryUsername) { "username mismatch: ${e.username}" }
        check(e.password == entryPassword) { "password mismatch" }
        check(e.url == entryUrl) { "url mismatch: ${e.url}" }
        check(e.otp == null) { "otp should be null" }
        line("PASS: findEntry('Demo Login') 全字段读回一致")

        // --- ③ list_entry_summaries（摘要不含密码；has_otp 正确）---
        val sums = vault.listEntrySummaries().sortedBy { it.title }
        check(sums.size == 2) { "expected 2 summaries, got ${sums.size}" }
        check(sums[0].title == "Demo Login" && !sums[0].hasOtp && sums[0].username == entryUsername)
        check(sums[1].title == "Demo OTP" && sums[1].hasOtp)
        line("PASS: listEntrySummaries() 2 条；密码不出现在摘要面")

        // --- ④ entry_totp_now（RFC 6238 参考密钥；时间相关，断言形状）---
        val code = vault.entryTotpNow("Demo OTP")
        check(code.length == 6 && code.all { it.isDigit() }) { "bad totp code: $code" }
        line("PASS: entryTotpNow('Demo OTP') -> 6 位数字码")

        // --- ⑤ NotFound 语义 ---
        val nf = try {
            vault.findEntry("missing"); null
        } catch (ex: FfiException.NotFound) {
            ex
        }
        check(nf != null) { "expected FfiException.NotFound" }
        line("PASS: findEntry(missing) -> FfiException.NotFound")

        // --- ⑤b DR-012 A1：锁库后 saveSession 被拒（凭据因子随 actor drop 清零）---
        vault.lock()
        val denied = try {
            vault.saveSession(fixture.absolutePath); null
        } catch (ex: FfiException.Format) {
            ex
        }
        check(denied != null) { "expected FfiException.Format from saveSession after lock, got $denied" }
        line("PASS: 锁库后 saveSession 被拒（Format unavailable，因子已清零）")

        vault.close()

        // --- ⑥ 错密码 → Credentials（凭据语义不细化）---
        val cred = try {
            openDb(fixture.absolutePath, "definitely-wrong", null); null
        } catch (ex: FfiException.Credentials) {
            ex
        }
        check(cred != null) { "expected FfiException.Credentials" }
        line("PASS: 错密码 -> FfiException.Credentials")

        // --- ⑦ KDF 复测（冷/热差异观察；3 次取稳定值）---
        repeat(2) { i ->
            val t = SystemClock.elapsedRealtime()
            val v = openDb(fixture.absolutePath, dbPassword, null)
            val ms = SystemClock.elapsedRealtime() - t
            val h = vmHwmKb()
            v.close()
            line("PASS: openDb #${i + 2} (reopen) ${ms} ms; VmHWM $h kB")
        }
        line("== ALL E2E CHECKS PASSED ==")

        // --- ⑧ 首建校准链路（DR-010/011 终态；init_vault=true 时执行）---
        // 独立探针文件，不覆盖壳层库；走 VaultSession.createVault（UnlockActivity 同一代码路径）
        if (initVault) {
            val probe = File(filesDir, "init_probe.kdbx")
            probe.delete()
            val t = SystemClock.elapsedRealtime()
            val (v, p) = VaultSession.createVault(probe.absolutePath, dbPassword, null)
            val ms = SystemClock.elapsedRealtime() - t
            check(probe.exists()) { "probe vault not created" }
            check(p.memoryKib == 64u * 1024u) { "calibrated m out of DR-010 floor: ${p.memoryKib}" }
            check(p.iterations in 2u..8u) { "calibrated t out of DR-010 bounds: ${p.iterations}" }
            check(p.parallelism == 4u) { "calibrated p != 4: ${p.parallelism}" }
            v.listEntrySummaries() // 已解锁可读（空库 → 空摘要）
            line("PASS: createVault 校准建链路 ${ms} ms（m=${p.memoryKib} t=${p.iterations} p=${p.parallelism}，DR-010 边界内）")
            probe.delete() // 探针库用后即删（壳层不遗留测试产物）
        }

        // --- ⑧b DR-012 A1：saveSession 保存闭环（add → save_session → 锁库 → 重开读回）---
        // 探针库从 fixture 复制（不改壳层库文件）；等价于 SaveConfirmActivity 写入路径的 FFI 面
        val saveProbe = File(filesDir, "save_probe.kdbx")
        saveProbe.delete()
        fixture.copyTo(saveProbe, overwrite = true)
        run {
            val v = openDb(saveProbe.absolutePath, dbPassword, null)
            v.addEntry(
                uniffi.sp_kffi.FfiEntryDraft(
                    title = "Autofill Saved",
                    username = entryUsername,
                    password = entryPassword,
                    url = entryUrl,
                    notes = null,
                    otp = null,
                    customFields = emptyList(),
                    tags = emptyList(),
                ),
            )
            v.saveSession(saveProbe.absolutePath)
            v.lock()
        }
        run {
            val v2 = openDb(saveProbe.absolutePath, dbPassword, null)
            val saved = v2.findEntry("Autofill Saved")
            check(saved.password == entryPassword && saved.username == entryUsername) {
                "session-saved entry mismatch"
            }
            v2.lock()
        }
        saveProbe.delete() // 探针库用后即删
        line("PASS: saveSession 保存闭环（add → save_session → 重开读回，DR-012 A1）")

        // M1b 验证钩子：解锁 VaultSession（供 Autofill 演示链路使用；随后超时注入测试也依赖它）
        try {
            VaultSession.unlock(fixture.absolutePath, dbPassword, null)
            line("PASS: VaultSession unlocked (autofill hook)")
        } catch (t: Throwable) {
            line("WARN: session unlock failed: ${t::class.simpleName}")
        }

        // --- ⑨ 空闲锁库注入测试（SP-F-802 验收；idle_timeout_ms>0 时执行）---
        if (idleTimeoutMs > 0) {
            check(VaultSession.isUnlocked) { "session must be unlocked before idle test" }
            val deadline = SystemClock.elapsedRealtime() + idleTimeoutMs + 3000
            while (VaultSession.isUnlocked && SystemClock.elapsedRealtime() < deadline) {
                Thread.sleep(250)
            }
            check(!VaultSession.isUnlocked) { "session still unlocked after ${idleTimeoutMs}ms idle" }
            check(VaultSession.activeVault() == null) { "activeVault must be null after idle lock (secret ops rejected)" }
            line("PASS: idle timeout lock (injected ${idleTimeoutMs} ms)；锁后 activeVault()==null")
            VaultSession.idleTimeoutMs = VaultSession.DEFAULT_IDLE_TIMEOUT_MS
        }
        line("== M1 E2E DONE ==")
    }

    companion object {
        private const val TAG = "SPIKE"
    }
}
