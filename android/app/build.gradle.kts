plugins {
    id("com.android.application")
    kotlin("android")
}

android {
    namespace = "com.superpasswdkey.spike"
    compileSdk = 35
    buildToolsVersion = "36.0.0"

    defaultConfig {
        applicationId = "com.superpasswdkey.spike"
        minSdk = 28
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0-m1"
    }

    // spike 不做混淆/压缩，保持可读栈
    buildTypes {
        getByName("debug") { isMinifyEnabled = false }
        getByName("release") { isMinifyEnabled = false }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }

    // libsp_kffi.so 需解压到 nativeLibraryDir 供 JNA System.loadLibrary 查找
    packaging {
        jniLibs { useLegacyPackaging = true }
    }
}

dependencies {
    // UniFFI 生成的 Kotlin 绑定经 JNA 调用 Rust
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
