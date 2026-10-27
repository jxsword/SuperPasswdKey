package com.superpasswdkey.spike

import android.app.PendingIntent
import android.app.assist.AssistStructure
import android.content.Intent
import android.os.CancellationSignal
import android.service.autofill.AutofillService
import android.service.autofill.Dataset
import android.service.autofill.FillCallback
import android.service.autofill.FillRequest
import android.service.autofill.FillResponse
import android.service.autofill.SaveCallback
import android.service.autofill.SaveRequest
import android.text.InputType
import android.util.Log
import android.view.autofill.AutofillId
import android.view.autofill.AutofillValue
import android.widget.RemoteViews
import uniffi.sp_kffi.FfiEntrySummary
import uniffi.sp_kffi.Vault
import java.util.concurrent.Executors

/**
 * SP-F-803 系统 Autofill 服务（design_docs/14 §2/§4）。
 *
 * 填充路径：FillRequest → 表单启发式（密码框+邻近文本框）→ 页面标识：
 *   浏览器场景取 ViewNode.webDomain（WebView 内容，M1c 走真链路）；
 *   非浏览器场景用包名作为 host 精确匹配（宽匹配 backlog）；
 *   → rankEntryUrls（单源引擎 sp-core::match_url 经 FFI）→ 命中摘要后
 *   find_entry 单条取回 → Dataset（RemoteViews 仅标题+用户名，SP-F-804）。
 * 会话锁定：Dataset 挂 UnlockActivity 认证 Intent，解锁后经
 * EXTRA_AUTHENTICATION_RESULT 回填（SP-F-802）。
 * 机密边界（SP-F-804）：密码只进 Dataset，不落日志/存储。
 * 保存场景（Autofill save）为 backlog（design_docs/14 §6.4）。
 */
class SpAutofillService : AutofillService() {
    private val io = Executors.newSingleThreadExecutor { r -> Thread(r, "sp-autofill") }

    override fun onFillRequest(request: FillRequest, cancellationSignal: CancellationSignal, callback: FillCallback) {
        io.execute {
            runCatching { handleFill(request, callback) }
                .onFailure { Log.w(TAG, "fill request failed: ${it::class.simpleName}: ${it.message}") }
        }
    }

    private fun handleFill(request: FillRequest, callback: FillCallback) {
        val structure = request.fillContexts.lastOrNull()?.structure
        if (structure == null) {
            callback.onSuccess(null)
            return
        }
        val form = findLoginForm(structure)
        if (form == null) {
            Log.i(TAG, "fill: no login form detected")
            callback.onSuccess(null)
            return
        }
        // 页面标识：webDomain（浏览器）> 调用方包名（非浏览器，design_docs/14 §3 MVP 口径）
        val targetPackage = structure.activityComponent?.packageName
        val pageUrl = extractPageUrl(structure) ?: ("https://" + (targetPackage ?: "unknown"))
        Log.i(TAG, "fill: page=$pageUrl")

        val session = VaultSession.activeVault()
        if (session == null) {
            // 会话锁定：Dataset 挂 UnlockActivity 认证，解锁后经 EXTRA_AUTHENTICATION_RESULT 回填（SP-F-802）
            Log.i(TAG, "fill: session locked -> auth dataset")
            callback.onSuccess(
                FormMatcher.buildAuthResponse(applicationContext, form.usernameId, form.passwordId, pageUrl),
            )
            return
        }
        val summaries = session.listEntrySummaries()
        val ranked = FormMatcher.rank(pageUrl, summaries)
        if (ranked.isEmpty()) {
            Log.i(TAG, "fill: no matching entries for $pageUrl")
            callback.onSuccess(null) // 不匹配返回空集不猜测（SP-F-803 验收）
            return
        }
        Log.i(TAG, "fill: matched ${ranked.map { it.title }}")
        callback.onSuccess(
            FormMatcher.buildResponse(applicationContext, session, ranked, form.usernameId, form.passwordId),
        )
    }

    /** 页面标识：WebView 节点的 webDomain（浏览器）→ https://domain；否则 null（包名匹配） */
    private fun extractPageUrl(structure: AssistStructure): String? {
        var found: String? = null
        var debugCount = 0
        fun walk(node: AssistStructure.ViewNode) {
            if (node.webDomain != null && found == null) found = node.webDomain
            if (debugCount < 30) {
                Log.d(
                    TAG,
                    "node[${debugCount}] cls=${node.className} web=${node.webDomain} " +
                        "hints=${node.autofillHints?.joinToString(",")} hint=${node.hint} " +
                        "inputType=${node.inputType} pass=${node.inputType and android.text.InputType.TYPE_CLASS_TEXT != 0 && node.inputType and android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD != 0}",
                )
                debugCount++
            }
            for (i in 0 until node.childCount) walk(node.getChildAt(i))
        }
        for (w in 0 until structure.windowNodeCount) {
            walk(structure.getWindowNodeAt(w).rootViewNode)
        }
        return found?.let { "https://$it" }
    }

    data class LoginForm(val usernameId: AutofillId, val passwordId: AutofillId)

    /** 登录表单启发式：密码框（inputType/hint）+ 邻近文本框（design_docs/14 §5 R14-1 MVP 口径） */
    private fun findLoginForm(structure: AssistStructure): LoginForm? {
        var usernameId: AutofillId? = null
        var passwordId: AutofillId? = null
        val textBeforePassword = mutableListOf<AutofillId>()

        fun walk(node: AssistStructure.ViewNode) {
            node.autofillId?.let { id ->
                val hints = node.autofillHints?.toList().orEmpty()
                val it0 = node.inputType
                val isPassword =
                    (it0 and InputType.TYPE_CLASS_TEXT != 0 && it0 and InputType.TYPE_TEXT_VARIATION_PASSWORD != 0) ||
                        (it0 and InputType.TYPE_CLASS_NUMBER != 0 && it0 and InputType.TYPE_NUMBER_VARIATION_PASSWORD != 0)
                when {
                    hints.any { it.contains("PASSWORD", true) } || isPassword -> {
                        if (passwordId == null) passwordId = id
                    }
                    hints.any { h -> h.contains("USERNAME", true) || h.contains("EMAIL", true) || h.contains("USER", true) } -> {
                        if (usernameId == null) usernameId = id
                    }
                    it0 and InputType.TYPE_CLASS_TEXT != 0 ->
                        textBeforePassword.add(id)
                }
            }
            for (i in 0 until node.childCount) walk(node.getChildAt(i))
        }
        for (w in 0 until structure.windowNodeCount) {
            walk(structure.getWindowNodeAt(w).rootViewNode)
        }
        // 无显式 hint 时：密码框之前最后一个文本框视为用户名
        if (usernameId == null && passwordId != null) {
            usernameId = textBeforePassword.lastOrNull()
        }
        val u = usernameId ?: return null
        val p = passwordId ?: return null
        return LoginForm(u, p)
    }

    override fun onSaveRequest(request: SaveRequest, callback: SaveCallback) {
        // Autofill 保存场景（新密码入库）为 backlog——design_docs/14 §6.4
        callback.onSuccess()
    }

    companion object {
        private const val TAG = "SPIKE"
    }
}

/** 匹配与响应构造（服务与 UnlockActivity 认证回路共用） */
object FormMatcher {
    private const val TAG = "SPIKE"
    private const val MAX_DATASETS = 5

    /**
     * 单源匹配：浏览器页面 → rankEntryUrls（SP-F-706 语义）；
     * 非浏览器（页面=包名 host）→ 条目 url 裸 host 精确匹配（宽匹配 backlog）。
     */
    fun rank(pageUrl: String, summaries: List<FfiEntrySummary>): List<FfiEntrySummary> {
        if (summaries.isEmpty()) return emptyList()
        val urls: List<String?> = summaries.map { it.url }
        return uniffi.sp_kffi.rankEntryUrls(pageUrl, urls).map { summaries[it.toInt()] }
    }

    /** 命中条目构造 Dataset：find_entry 单条取回密码（即用即弃，SP-F-804） */
    fun buildResponse(
        appContext: android.content.Context,
        session: Vault?,
        ranked: List<FfiEntrySummary>,
        usernameId: AutofillId,
        passwordId: AutofillId,
    ): FillResponse {
        val builder = FillResponse.Builder()
        val v = session ?: return builder.build() // 无会话 → 空响应（不应发生：锁定走认证回路）
        for (s in ranked.take(MAX_DATASETS)) {
            val entry = runCatching { v.findEntry(s.title) }.getOrNull() ?: continue
            val presentation = RemoteViews(appContext.packageName, R.layout.dataset_item).apply {
                setTextViewText(R.id.dataset_title, s.title)
                setTextViewText(R.id.dataset_username, s.username ?: "")
            }
            val dataset = Dataset.Builder(presentation)
                .setValue(usernameId, AutofillValue.forText(entry.username ?: ""))
                .setValue(passwordId, AutofillValue.forText(entry.password ?: ""))
                .build()
            builder.addDataset(dataset)
        }
        return builder.build()
    }

    /** 锁定态：构造仅含认证回路的响应（UnlockActivity 解锁后回填） */
    fun buildAuthResponse(
        appContext: android.content.Context,
        usernameId: AutofillId,
        passwordId: AutofillId,
        pageUrl: String,
    ): FillResponse {
        val authIntent = Intent(appContext, UnlockActivity::class.java).apply {
            putExtra(UnlockActivity.EXTRA_PAGE_URL, pageUrl)
            putExtra(UnlockActivity.EXTRA_USERNAME_ID, usernameId)
            putExtra(UnlockActivity.EXTRA_PASSWORD_ID, passwordId)
        }
        val pending = PendingIntent.getActivity(
            appContext, 0, authIntent,
            PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        return FillResponse.Builder()
            .setAuthentication(arrayOf(usernameId, passwordId), pending.intentSender, null as android.widget.RemoteViews?)
            .build()
    }
}
