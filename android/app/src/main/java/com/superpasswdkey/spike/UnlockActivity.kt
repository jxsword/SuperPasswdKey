package com.superpasswdkey.spike

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.autofill.AutofillId
import android.view.autofill.AutofillManager
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import uniffi.sp_kffi.FfiException

/**
 * SP-F-802 解锁 Activity（design_docs/14 §2/§4）。双职责：
 *  1. 解锁/首次初始化：库文件存在 → 解锁表单；缺失 → 首建表单（主密码+确认+可选
 *     keyfile），经 VaultSession.createVault 走 calibrate_kdf 校准建库（DR-010/011
 *     终态：工作线程执行数秒，界面全程提示），成功后回显校准参数；
 *  2. Autofill 认证回路：携带 EXTRA_PAGE_URL/EXTRA_USERNAME_ID/EXTRA_PASSWORD_ID
 *     （SpAutofillService.buildAuthResponse），解锁/首建成功后经
 *     EXTRA_AUTHENTICATION_RESULT 返回 FillResponse（SP-F-803 标准回路）。
 * 凭据纪律（SP-F-804）：只进 FFI 调用栈，不落 SharedPreferences/文件/日志；
 * 错误呈现仅映射类别，不回显任何凭据值。
 */
class UnlockActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var actionBtn: Button
    private var pageUrl: String? = null
    private var usernameId: AutofillId? = null
    private var passwordId: AutofillId? = null

    private val isAuthLoop: Boolean
        get() = pageUrl != null && usernameId != null && passwordId != null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        pageUrl = intent.getStringExtra(EXTRA_PAGE_URL)
        usernameId = intent.getParcelableExtra(EXTRA_USERNAME_ID)
        passwordId = intent.getParcelableExtra(EXTRA_PASSWORD_ID)

        val dbFile = VaultSession.vaultPath(filesDir)
        val isFirstInit = !dbFile.exists()

        status = TextView(this)
        val password = EditText(this).apply {
            hint = "主密码"
            inputType = android.text.InputType.TYPE_CLASS_TEXT or
                android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD
        }
        val confirm = EditText(this).apply {
            hint = "确认主密码"
            inputType = android.text.InputType.TYPE_CLASS_TEXT or
                android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD
        }
        val keyfile = EditText(this).apply { hint = "keyfile 路径（可选）" }
        actionBtn = Button(this).apply {
            text = when {
                isFirstInit && isAuthLoop -> "创建库并填充"
                isFirstInit -> "创建库并解锁"
                isAuthLoop -> "解锁并填充"
                else -> "解锁"
            }
        }
        if (isFirstInit) {
            actionBtn.setOnClickListener {
                val pw = password.text.toString()
                if (pw.isEmpty() || pw != confirm.text.toString()) {
                    status.text = "主密码为空或两次输入不一致"
                    return@setOnClickListener
                }
                // pw/keyfile 只进后台闭包 → FFI 调用栈（SP-F-804）
                val kf = keyfile.text.toString().trim().takeIf { it.isNotEmpty() }
                runAsync(
                    op = "first-init",
                    hint = "首次初始化：KDF 校准 + 建库中（约 5–10 秒，请勿退出）…",
                ) {
                    val (_, p) = VaultSession.createVault(dbFile.absolutePath, pw, kf)
                    "已创建并解锁（校准参数 m=${p.memoryKib / 1024u}MiB t=${p.iterations} p=${p.parallelism}）"
                }
            }
        } else {
            confirm.visibility = View.GONE
            actionBtn.setOnClickListener {
                val pw = password.text.toString()
                val kf = keyfile.text.toString().trim().takeIf { it.isNotEmpty() }
                runAsync(op = "unlock", hint = "解锁中（KDF 派生，约 1–3 秒）…") {
                    VaultSession.unlock(dbFile.absolutePath, pw, kf)
                    "已解锁"
                }
            }
        }

        setContentView(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(48, 48, 48, 48)
                gravity = Gravity.CENTER_VERTICAL
                addView(
                    TextView(this@UnlockActivity).apply {
                        text = if (isFirstInit) {
                            "首次初始化：将实测校准 KDF 参数（m=64MiB/p=4，t∈[2,8] 目标 ~1s）并创建 ${dbFile.name}"
                        } else {
                            "解锁库：${dbFile.name}"
                        }
                    },
                )
                addView(status)
                addView(password)
                addView(confirm)
                addView(keyfile)
                addView(actionBtn)
            },
        )
        if (isAuthLoop) Log.i(TAG, "unlock activity opened in auth-loop mode")
    }

    override fun onResume() {
        super.onResume()
        refreshSessionStatus()
    }

    /** 锁定状态显示（SP-F-802 正式化项） */
    private fun refreshSessionStatus() {
        status.text = when {
            VaultSession.isUnlocked -> "会话状态：已解锁"
            VaultSession.vaultPath(filesDir).exists() -> "会话状态：已锁定"
            else -> "会话状态：无库（首次初始化）"
        }
    }

    /** 后台线程执行 + UI 提示骨架：KDF/校准均阻塞数秒，按钮期间禁用防重复提交 */
    private fun runAsync(op: String, hint: String, action: () -> String) {
        status.text = hint
        actionBtn.isEnabled = false
        Thread {
            val outcome = runCatching { action() }
            runOnUiThread {
                actionBtn.isEnabled = true
                outcome.fold(
                    onSuccess = { msg ->
                        status.text = msg
                        Toast.makeText(this, msg, Toast.LENGTH_SHORT).show()
                        finishAuthLoop()
                    },
                    onFailure = { e ->
                        Log.i(TAG, "$op failed: ${e::class.simpleName}") // 只记错误类别，不记凭据
                        val msg = describe(e)
                        status.text = msg
                        Toast.makeText(this, msg, Toast.LENGTH_LONG).show()
                    },
                )
            }
        }.start()
    }

    /** FfiException 分类提示（SP-F-802 错误呈现正式化）；只映射类别，不携带凭据值 */
    private fun describe(e: Throwable): String = when (e) {
        is FfiException.Credentials -> "解锁失败：主密码或 keyfile 不正确"
        is FfiException.Io -> "失败：库文件读取/写入错误（IO）"
        is FfiException.Format -> "失败：库文件格式错误或已损坏"
        is FfiException.NotFound -> "失败：条目不存在"
        is FfiException.Parameter -> "失败：参数错误"
        else -> "失败：${e::class.simpleName ?: "unknown"}"
    }

    /** Autofill 认证回路：重建 FillResponse 返回系统（SP-F-803）；手动解锁路径无 extra 直接返回 */
    private fun finishAuthLoop() {
        val pageUrl = pageUrl ?: return
        val usernameId = usernameId ?: return
        val passwordId = passwordId ?: return
        val session = VaultSession.activeVault() ?: return
        val ranked = FormMatcher.rank(pageUrl, session.listEntrySummaries())
        val response = if (ranked.isEmpty()) {
            null
        } else {
            FormMatcher.buildResponse(applicationContext, session, ranked, usernameId, passwordId)
        }
        val resultIntent = Intent().putExtra(AutofillManager.EXTRA_AUTHENTICATION_RESULT, response)
        setResult(RESULT_OK, resultIntent)
        Log.i(TAG, "auth loop: response built (${ranked.size} datasets)")
        finish()
    }

    companion object {
        private const val TAG = "SPIKE"
        const val EXTRA_PAGE_URL = "sp.page_url"
        const val EXTRA_USERNAME_ID = "sp.username_id"
        const val EXTRA_PASSWORD_ID = "sp.password_id"
    }
}
