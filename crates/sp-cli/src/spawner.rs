//! 子进程启动器 —— F8（临时环境变量注入）的唯一进程生成点。
//!
//! 语义与安全边界（design_docs/04）：
//! - 程序名来自用户在命令行显式指定的 `sp run -- <cmd>`，等价 1Password `op run`；
//! - 以 argv 数组直接 exec：无 shell、无字符串拼接——程序名即用户意图本体，
//!   不是拼接入既有命令的不可信数据，不存在 shell 注入面；
//! - 环境变量键值对注入子进程，父 shell 不参与，子进程退出即销毁。

use std::process::{Child, Stdio};
use zeroize::Zeroizing;

/// 启动子进程（argv 直 exec）。stdin 恒继承；capture=true 时 stdout/stderr
/// 经遮蔽过滤器（design_docs/04 §2）转发。
pub fn spawn_child(
    program: &str,
    args: &[String],
    envs: &[(String, Zeroizing<String>)],
    capture: bool,
) -> std::io::Result<Child> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v.as_str());
    }
    cmd.stdin(Stdio::inherit())
        .stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stderr(if capture {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });
    cmd.spawn()
}
