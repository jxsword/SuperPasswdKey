package com.superpasswdkey.spike

import android.app.Activity
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.util.Log
import android.view.Gravity
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView

/**
 * SP-F-803 演示登录页（M1b 验收载体）：username/password 双字段表单。
 * 验证方式：系统 Autofill 弹出 SpAutofillService 数据集 → 选择后字段被填充。
 * SP-F-805 保存链路演示（DR-012 A1）："登录"= autofillManager.commit() 触发系统
 * 保存建议 → SaveConfirmActivity 确认入库；"提交（不保存）"= cancel() 取消会话
 * （系统不发起保存）。观测：TextWatcher 仅记录"哪个字段被填充了多长的值"，
 * **不记录密码值本身**（SP-F-804）。
 */
class DemoLoginActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val header = TextView(this).apply {
            text = "SP Autofill 演示登录页\n（聚焦字段触发系统 Autofill，选择 SP 数据集填充；" +
                "输入新凭据后点\"登录\"触发保存建议）"
            setPadding(32, 32, 32, 0)
        }
        val username = EditText(this).apply {
            hint = "用户名"
            setId(android.R.id.text1)
        }
        val password = EditText(this).apply {
            hint = "密码"
            inputType = android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD
        }
        password.addTextChangedListener(fieldLogger("password"))
        username.addTextChangedListener(fieldLogger("username"))
        val loginBtn = Button(this).apply {
            text = "登录"
            setOnClickListener {
                android.widget.Toast.makeText(
                    this@DemoLoginActivity,
                    "login submitted (demo only)",
                    android.widget.Toast.LENGTH_SHORT,
                ).show()
                autofillManagerCommit()
            }
        }
        val cancelBtn = Button(this).apply {
            text = "提交（不保存）"
            setOnClickListener {
                android.widget.Toast.makeText(
                    this@DemoLoginActivity,
                    "cancelled — system will not offer save",
                    android.widget.Toast.LENGTH_SHORT,
                ).show()
                autofillManagerCancel()
            }
        }
        setContentView(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(48, 96, 48, 48)
                gravity = Gravity.CENTER_HORIZONTAL
                addView(header)
                addView(username)
                addView(password)
                addView(loginBtn)
                addView(cancelBtn)
            },
        )
    }

    /** 填充观测：仅记字段名与值长度；密码明文绝不落日志（SP-F-804 代码审查口径） */
    private fun fieldLogger(name: String): TextWatcher = object : TextWatcher {
        override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
        override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
        override fun afterTextChanged(s: Editable?) {
            val len = s?.length ?: 0
            if (len > 0) Log.i(TAG, "AUTOFILLED/INPUT: $name len=$len")
        }
    }

    /** 系统 Autofill 会话提交/取消（触发 SP-F-805 保存建议 / 显式不保存） */
    private fun autofillManagerCommit() {
        getSystemService(android.view.autofill.AutofillManager::class.java)?.commit()
    }

    private fun autofillManagerCancel() {
        getSystemService(android.view.autofill.AutofillManager::class.java)?.cancel()
    }

    companion object {
        private const val TAG = "SPIKE"
    }
}
