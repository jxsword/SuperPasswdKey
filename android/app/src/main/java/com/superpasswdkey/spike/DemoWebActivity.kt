package com.superpasswdkey.spike

import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.webkit.WebView

/**
 * SP-F-803 M1c：webDomain 路径验证载体（design_docs/14 §6.3）。
 * 优先经 intent extra `web_url` 做真实导航（配合 `adb reverse` + 本机 HTTP 服务，
 * Chromium 才会上报 webDomain）；无 extra 时回退 loadDataWithBaseURL（webDomain 为空，
 * 走包名匹配——留作对照）。与浏览器（Chrome）场景同构地走
 * "webDomain → rankEntryUrls → Dataset" 链路。
 */
class DemoWebActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val url = intent.getStringExtra("web_url")
        val dbPassword = intent.getStringExtra("db_password")
        val webView = WebView(this).apply {
            settings.javaScriptEnabled = true // 供 evaluateJavascript 聚焦表单字段（M1c 验证用）
            webViewClient = object : android.webkit.WebViewClient() {
                override fun onPageFinished(view: WebView, url: String) {
                    Log.i(TAG, "page finished: $url")
                    // 确定性触发：JS 聚焦 username 字段（Chromium 随之向 autofill 框架上报虚拟视图进入）
                    view.evaluateJavascript(
                        "document.getElementById('u').focus(); document.getElementById('u').value=''; void 0",
                        null,
                    )
                    // 确定性触发（等价于聚焦时框架的行为）：解锁会话后主动请求填充
                    android.os.Handler(mainLooper).postDelayed(
                        {
                            val ensureSession = Runnable {
                                if (!VaultSession.isUnlocked && dbPassword != null) {
                                    try {
                                        VaultSession.unlock(
                                            java.io.File(filesDir, "test_vault.kdbx").absolutePath,
                                            dbPassword,
                                            null,
                                        )
                                    } catch (e: Throwable) {
                                        Log.i(TAG, "web demo unlock failed: ${e::class.simpleName}")
                                    }
                                }
                                val afm = getSystemService(android.view.autofill.AutofillManager::class.java)
                                afm?.requestAutofill(view)
                                Log.i(TAG, "requestAutofill sent")
                            }
                            Thread(ensureSession).start()
                        },
                        800,
                    )
                }
            }
            if (url != null) {
                Log.i(TAG, "loading real navigation: $url")
                loadUrl(url)
            } else {
                val html = """
                    <html><body style='font-family:sans-serif;padding:16px'>
                    <h3>演示登录（webDomain 路径）</h3>
                    <form>
                      <input id='u' autocomplete='username' placeholder='用户名'
                             style='width:90%;padding:10px;margin:6px 0;display:block'/>
                      <input id='p' type='password' autocomplete='current-password' placeholder='密码'
                             style='width:90%;padding:10px;margin:6px 0;display:block'/>
                    </form>
                    </body></html>
                """.trimIndent()
                loadDataWithBaseURL("https://login.example.com/", html, "text/html", "utf-8", null)
            }
        }
        setContentView(webView)
        Log.i(TAG, "demo web activity ready (url=$url)")
    }

    companion object {
        private const val TAG = "SPIKE"
    }
}
