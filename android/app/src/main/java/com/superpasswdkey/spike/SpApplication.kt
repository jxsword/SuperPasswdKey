package com.superpasswdkey.spike

import android.app.Application

/**
 * 进程级初始化：加载 Rust 内核 cdylib（libsp_kffi.so，JNA 随后按名链接）。
 * Autofill 服务可能先于任何 Activity 运行，加载必须挂在 Application.onCreate
 * 这一进程内最早确定时机（spike 时代依赖 MainActivity 先启动，正式化后兜底）。
 */
class SpApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        System.loadLibrary("sp_kffi")
    }
}
