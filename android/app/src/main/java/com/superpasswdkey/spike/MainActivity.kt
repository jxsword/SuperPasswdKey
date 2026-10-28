package com.superpasswdkey.spike

import android.app.Activity
import android.os.Bundle
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import java.io.File

/**
 * M1 产品壳入口（M1 正式化）：会话状态展示 + 功能入口。
 * e2e harness 已分离至 TestActivity——本类不再自动跑断言、不再接收凭据 extras
 * （MainActivity 历史契约 `.MainActivity --es db_password ...` 迁移至 TestActivity）。
 * 演示/测试载体（DemoLoginActivity/DemoWebActivity/TestActivity）在 M1 壳内保留入口，
 * 供 Autofill 验收链路使用（design_docs/14 §6）；正式 UI（条目管理/设置）为后续里程碑。
 */
class MainActivity : Activity() {
    private lateinit var status: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        status = TextView(this).apply { setPadding(32, 48, 32, 16) }
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(status)
        }
        setContentView(ScrollView(this).apply { addView(column) })

        addButton(column, "解锁 / 首次初始化库") {
            startActivity(android.content.Intent(this, UnlockActivity::class.java))
        }
        addButton(column, "锁定会话") {
            VaultSession.lockNow()
            refreshStatus()
            Toast.makeText(this, "已锁定", Toast.LENGTH_SHORT).show()
        }
        addButton(column, "演示登录页（Autofill 包名路径）") {
            startActivity(android.content.Intent(this, DemoLoginActivity::class.java))
        }
        addButton(column, "WebView 演示页（Autofill webDomain 路径）") {
            startActivity(android.content.Intent(this, DemoWebActivity::class.java))
        }
        addButton(column, "运行 e2e 测试（需 fixture/extras，见 TestActivity 文档）") {
            startActivity(android.content.Intent(this, TestActivity::class.java))
        }
    }

    override fun onResume() {
        super.onResume()
        refreshStatus()
    }

    private fun refreshStatus() {
        val db = VaultSession.vaultPath(filesDir)
        status.text = buildString {
            appendLine("SuperPasswdKey M1 壳（SP-F-802/803）")
            appendLine(
                "库文件：${if (db.exists()) "${db.name}（${File(db.path).length()} B）" else "不存在（待首次初始化）"}",
            )
            appendLine("会话：${if (VaultSession.isUnlocked) "已解锁" else "已锁定"}")
            appendLine("空闲锁库：${VaultSession.idleTimeoutMs / 1000}s")
        }
    }

    private fun addButton(column: LinearLayout, label: String, onClick: () -> Unit) {
        column.addView(
            Button(this).apply {
                text = label
                setOnClickListener { onClick() }
            },
        )
    }
}
