package com.superpasswdkey.spike

import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import uniffi.sp_kffi.FfiEntryDraft
import uniffi.sp_kffi.FfiEntrySummary
import uniffi.sp_kffi.FfiException
import uniffi.sp_kffi.Vault

/**
 * SP-F-805 保存确认页（DR-012 A1；design_docs/14 §7）：
 * SpAutofillService.onSaveRequest 经 SaveCallback.onSuccess(intentSender) 唤起，
 * 展示 host/包名 + 用户名；密码不回显——值只经 save intent extra 传递（系统 RAM，
 * 页面消费后不再持有，不落盘/日志，SP-F-804 审查口径如实声明）。
 * 流程：确认 → 归属（webDomain/包名，同解锁填充口径）→ 去重（同 host+用户名 →
 * 更新密码进 history；否则新增，标题=host，撞题时加 "(username)" 后缀保 find_entry
 * 单值语义）→ add_entry/edit_entry + saveSession（复用 actor 驻留因子，DR-012 A1）。
 * 会话锁定时页内提供主密码解锁（密码库口径；keyfile 库需先经解锁页解锁会话，
 * 保存确认页不做 keyfile 输入）。取消/返回 → 直接 finish，系统按取消处理（不写库）。
 */
class SaveConfirmActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var unlockField: EditText
    private lateinit var confirmBtn: Button
    private var pageUrl: String = ""
    private var username: String? = null
    private var password: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        pageUrl = intent.getStringExtra(EXTRA_PAGE_URL) ?: ""
        username = intent.getStringExtra(EXTRA_USERNAME)
        password = intent.getStringExtra(EXTRA_SECRET)
        if (pageUrl.isEmpty() || password.isNullOrEmpty()) {
            Log.i(TAG, "save confirm: missing target or empty secret field — cancel")
            finish()
            return
        }
        val host = hostOf(pageUrl)

        status = TextView(this)
        unlockField = EditText(this).apply {
            hint = "主密码（库已锁定时解锁用）"
            inputType = android.text.InputType.TYPE_CLASS_TEXT or
                android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD
        }
        confirmBtn = Button(this).apply { text = "保存到库" }
        confirmBtn.setOnClickListener { onConfirm() }
        val cancelBtn = Button(this).apply {
            text = "不保存"
            setOnClickListener {
                Log.i(TAG, "save confirm: user declined")
                finish()
            }
        }

        setContentView(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(48, 96, 48, 48)
                gravity = Gravity.CENTER_HORIZONTAL
                addView(TextView(this@SaveConfirmActivity).apply {
                    text = "将以下凭据保存到 SuperPasswdKey？\n\n站点/应用：$host\n用户名：${username ?: "（空）"}\n密码：••••••••"
                })
                addView(status)
                addView(unlockField)
                refreshUnlockVisibility()
                addView(confirmBtn)
                addView(cancelBtn)
            },
        )
    }

    /** 已解锁会话隐藏解锁框；锁定时显示（onCreate 一次性判定，超时竞态由 onConfirm 兜底） */
    private fun refreshUnlockVisibility() {
        unlockField.visibility = if (VaultSession.isUnlocked) View.GONE else View.VISIBLE
    }

    private fun onConfirm() {
        val unlockPw = unlockField.text.toString()
        status.text = "保存中…"
        confirmBtn.isEnabled = false
        Thread {
            val outcome = runCatching {
                val session = VaultSession.activeVault()
                    ?: VaultSession.unlock(
                        VaultSession.vaultPath(filesDir).absolutePath,
                        unlockPw,
                        null,
                    )
                writeToVault(session)
                session.saveSession(VaultSession.vaultPath(filesDir).absolutePath)
                VaultSession.touch()
            }
            runOnUiThread {
                outcome.fold(
                    onSuccess = {
                        Log.i(TAG, "save confirm: saved for ${hostOf(pageUrl)}")
                        Toast.makeText(this, "已保存：${hostOf(pageUrl)}", Toast.LENGTH_SHORT).show()
                        finish() // 确认页 finish → 系统 save 回路完成
                    },
                    onFailure = { e ->
                        Log.i(TAG, "save failed: ${e::class.simpleName}") // 只记类别，不记凭据
                        status.text = when (e) {
                            is FfiException.Credentials -> "解锁失败：主密码不正确"
                            is FfiException.Format -> "保存失败：库格式错误或会话不可用"
                            is FfiException.Parameter -> "保存失败：条目参数不合法（如标题为空）"
                            else -> "保存失败：${e::class.simpleName ?: "unknown"}"
                        }
                        // 凭据失败大概率是确认期间会话超时锁定 → 露出解锁框供重试
                        if (e is FfiException.Credentials) refreshUnlockVisibility()
                        confirmBtn.isEnabled = true
                    },
                )
            }
        }.start()
    }

    /** 归属 + 去重 + 写入（后台线程调用；session 须已解锁） */
    private fun writeToVault(session: Vault) {
        val host = hostOf(pageUrl)
        val summaries = session.listEntrySummaries()
        val existing = summaries.firstOrNull { hostOf(it.url) == host && it.username == username }
        val draft = FfiEntryDraft(
            title = existing?.title ?: uniqueTitle(host, summaries),
            username = username?.takeIf { it.isNotEmpty() },
            password = password,
            url = pageUrl,
            notes = null,
            otp = null,
            customFields = emptyList(),
        )
        if (existing == null) {
            session.addEntry(draft)
            Log.i(TAG, "save: add entry '$host'")
        } else {
            // 同 host+用户名已存在 → 更新（旧快照进 history，语义 = sp_kdbx::edit_entry）
            session.editEntry(existing.title, draft)
            Log.i(TAG, "save: update existing entry '${existing.title}'")
        }
    }

    private fun uniqueTitle(host: String, summaries: List<FfiEntrySummary>): String =
        if (summaries.none { it.title == host }) host else "$host ($username)"

    companion object {
        private const val TAG = "SPIKE"
        const val EXTRA_PAGE_URL = "sp.save_page_url"
        const val EXTRA_USERNAME = "sp.save_username"

        /** 凭据值字段 extra 键（值为运行期用户输入经系统 save 回路传入，非源码常量机密） */
        const val EXTRA_SECRET = "sp.save_secret"

        /** 归属 host 口径：strip scheme 后取路径前段（库内 url 均为本服务写出的 https://host 形态） */
        fun hostOf(url: String?): String =
            url?.removePrefix("https://")?.removePrefix("http://")?.substringBefore('/') ?: ""
    }
}
