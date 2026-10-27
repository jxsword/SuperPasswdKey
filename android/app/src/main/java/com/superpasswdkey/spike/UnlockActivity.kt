package com.superpasswdkey.spike

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.autofill.AutofillManager
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.Toast
import uniffi.sp_kffi.FfiException

/**
 * SP-F-802 解锁 Activity（design_docs/14 §2/§4）。
 * 双入口：
 *  1. 手动解锁：am start / 其他界面调用，成功后 Toast 并 finish；
 *  2. Autofill 认证回路：携带 EXTRA_PAGE_URL/EXTRA_USERNAME_ID/EXTRA_PASSWORD_ID
 *     （SpAutofillService.buildAuthResponse），解锁成功后经
 *     EXTRA_AUTHENTICATION_RESULT 返回 FillResponse（SP-F-803 标准回路）。
 * 凭据纪律：只进 FFI 调用栈，不落 SharedPreferences/文件/日志（SP-F-804）。
 */
class UnlockActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val pageUrl = intent.getStringExtra(EXTRA_PAGE_URL)
        val usernameId = intent.getParcelableExtra<android.view.autofill.AutofillId>(EXTRA_USERNAME_ID)
        val passwordId = intent.getParcelableExtra<android.view.autofill.AutofillId>(EXTRA_PASSWORD_ID)
        val isAuthLoop = pageUrl != null && usernameId != null && passwordId != null

        val password = EditText(this).apply {
            hint = "主密码"
            inputType = android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD
        }
        val keyfile = EditText(this).apply { hint = "keyfile 路径（可选）" }
        val unlockBtn = Button(this).apply {
            text = if (isAuthLoop) "解锁并填充" else "解锁"
            setOnClickListener { doUnlock(pageUrl, usernameId, passwordId, password, keyfile) }
        }
        setContentView(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(48, 48, 48, 48)
                gravity = Gravity.CENTER_VERTICAL
                addView(password)
                addView(keyfile)
                addView(unlockBtn)
            },
        )
        // 解锁后焦点回填触发（手动解锁场景无需处理）
        if (isAuthLoop) Log.i(TAG, "unlock activity opened in auth-loop mode")
    }

    private fun doUnlock(
        pageUrl: String?,
        usernameId: android.view.autofill.AutofillId?,
        passwordId: android.view.autofill.AutofillId?,
        password: EditText,
        keyfile: EditText,
    ) {
        val dbFile = java.io.File(filesDir, "test_vault.kdbx")
        val pw = password.text.toString()
        val kf = keyfile.text.toString().trim().takeIf { it.isNotEmpty() }
        Thread {
            val result = runCatching { VaultSession.unlock(dbFile.absolutePath, pw, kf) }
            runOnUiThread {
                result.fold(
                    onSuccess = {
                        Toast.makeText(this, "已解锁", Toast.LENGTH_SHORT).show()
                        finishAuthLoop(pageUrl, usernameId, passwordId)
                    },
                    onFailure = { e ->
                        val kind = (e as? FfiException)?.let { e::class.simpleName } ?: e::class.simpleName
                        Log.i(TAG, "unlock failed: $kind") // 只记错误类别，不记凭据
                        Toast.makeText(this, "解锁失败：$kind", Toast.LENGTH_LONG).show()
                    },
                )
            }
        }.start()
    }

    /** Autofill 认证回路：重建 FillResponse 返回系统（SP-F-803） */
    private fun finishAuthLoop(
        pageUrl: String?,
        usernameId: android.view.autofill.AutofillId?,
        passwordId: android.view.autofill.AutofillId?,
    ) {
        if (pageUrl == null || usernameId == null || passwordId == null) return
        val session = VaultSession.activeVault() ?: return
        val ranked = FormMatcher.rank(pageUrl, session.listEntrySummaries())
        val response = if (ranked.isEmpty()) null else
            FormMatcher.buildResponse(applicationContext, session, ranked, usernameId, passwordId)
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
