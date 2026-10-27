package com.superpasswdkey.spike

import android.app.Activity
import android.graphics.Typeface
import android.os.Bundle
import android.os.SystemClock
import android.util.Log
import android.widget.ScrollView
import android.widget.TextView
import uniffi.sp_kffi.FfiException
import uniffi.sp_kffi.openDb
import java.io.File

/**
 * Android spike e2e（DR-008）：加载 UniFFI 绑定，打开 KDBX4 测试库并断言条目读回一致，
 * 同时实测 64MiB Argon2id 的开库耗时与内存峰值（x86_64 模拟器，非真机）。
 *
 * 运行前置：
 *   1. cargo ndk -t x86_64-linux-android --platform 28 -- build -p sp-kffi
 *      并复制 target/x86_64-linux-android/debug/libsp_kffi.so 到 app/src/main/jniLibs/x86_64/
 *   2. 生成 fixture（口令经环境变量，不落源码）：
 *      SP_TEST_DB_PASSWORD=... SP_TEST_ENTRY_PASSWORD=... \
 *        cargo run -p sp-kffi --example make_spike_vault -- test_vault.kdbx
 *   3. 推送 fixture（scope storage 下 shell 无法直接写应用外部目录，走 run-as）：
 *      adb push test_vault.kdbx /data/local/tmp/
 *      adb shell run-as com.superpasswdkey.spike cp /data/local/tmp/test_vault.kdbx files/
 *   4. 启动（凭据经 intent extra 传入，不落 SharedPreferences/日志）：
 *      adb shell am start -n com.superpasswdkey.spike/.MainActivity \
 *        --es db_password ... --es entry_password ... --es entry_username demo-user
 * 结果：logcat -s SPIKE 或 run-as cat files/result.txt
 */
class MainActivity : Activity() {
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
        setContentView(ScrollView(this).apply { addView(tv) })
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
        line("== SuperPasswdKey Android spike e2e ==")
        System.loadLibrary("sp_kffi")

        val dbPassword = intent.getStringExtra("db_password")
        val entryPassword = intent.getStringExtra("entry_password")
        val entryUsername = intent.getStringExtra("entry_username") ?: "demo-user"
        if (dbPassword == null || entryPassword == null) {
            line("FAIL: missing intent extras db_password/entry_password")
            return
        }

        val fixture = File(filesDir, "test_vault.kdbx")
        if (!fixture.exists()) {
            line("PENDING: fixture missing — push test_vault.kdbx via run-as (see MainActivity doc)")
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
        check(e.url == "http://127.0.0.1:8080") { "url mismatch: ${e.url}" }
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
    }

    companion object {
        private const val TAG = "SPIKE"
    }
}
