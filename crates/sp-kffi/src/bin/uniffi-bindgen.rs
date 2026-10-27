//! uniffi-bindgen CLI 入口：从编译产物生成 Kotlin 绑定。
//! 用法：cargo run -p sp-kffi --bin uniffi-bindgen -- generate --library <libsp_kffi.so> --language kotlin --out-dir <dir>

fn main() {
    uniffi::uniffi_bindgen_main()
}
