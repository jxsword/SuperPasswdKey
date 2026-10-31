//! sp-kffi：UniFFI 绑定层（Android，DR-002/DR-008/DR-009）。
//!
//! 语义完全委托 sp-core + sp-kdbx，本 crate 不含业务逻辑与加密代码；
//! 仅做类型桥接（SpError → FfiError、EntryModel ↔ Ffi 类型）。
//!
//! 线程模型（design_docs/13 §2）：
//! keepass_ng 的 `Database` 内部为 Rc/RefCell 树（!Send + !Sync），而 UniFFI 0.32
//! 对导出对象强制 `Send + Sync`。解法 = **线程封闭 actor**：每个 Vault 拥有专用
//! 工作线程，Database 仅在该线程内被触碰；方法调用经 channel 派发并阻塞等结果。
//! 因此 Kotlin 侧可从任意线程调用（含并发），请求实际在工作线程上串行执行。

use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

use sp_core::{CredentialFactors, EntryModel, KdfParams, SecureString, SpError};

// ---------- FFI 类型面 ----------

/// Argon2id 参数（字段与 sp_core::KdfParams 一致，memory 单位 KiB，硬下限 64MiB）
#[derive(uniffi::Record, Debug, Clone, Copy)]
pub struct FfiKdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for FfiKdfParams {
    /// DR-010 终态已落地（DR-011）：新建库走 `calibrate_kdf`（t∈[2,8]/m=64MiB/p=4 校准至
    /// ~1s），本默认仅保留为**跳过校准的兜底**（测试 fixture、无校准条件的调用方）。
    /// m=64MiB 红线 #6 硬下限不动；t=4/p=4 中端真机解锁 ~1.1–1.3s。仅影响以默认参数
    /// 新建的库（Argon2 参数随 KDBX4 头存储，旧库/互操作不受影响）。
    fn default() -> Self {
        FfiKdfParams {
            memory_kib: 64 * 1024,
            iterations: 4,
            parallelism: 4,
        }
    }
}

impl From<KdfParams> for FfiKdfParams {
    fn from(p: KdfParams) -> Self {
        FfiKdfParams {
            memory_kib: p.memory_kib,
            iterations: p.iterations,
            parallelism: p.parallelism,
        }
    }
}

impl From<FfiKdfParams> for KdfParams {
    fn from(p: FfiKdfParams) -> Self {
        KdfParams {
            memory_kib: p.memory_kib,
            iterations: p.iterations,
            parallelism: p.parallelism,
        }
    }
}

/// 条目全文（读回；含密码——仅经单条显式取回；JNI String 无法零化，风险 R13-1）
#[derive(uniffi::Record, Debug, Clone)]
pub struct FfiEntry {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub otp: Option<String>,
    /// 标签分类（SP-F-108，S1b-2）
    pub tags: Vec<String>,
}

/// 条目摘要（不含密码/TOTP，对齐 sp_kdbx::EntrySummary / SP-F-706 语义）
#[derive(uniffi::Record, Debug, Clone)]
pub struct FfiEntrySummary {
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub has_otp: bool,
    /// 标签分类（SP-F-108，S1b-2）
    pub tags: Vec<String>,
}

/// 自定义字段（语义 = sp_core::CustomField；protected 的存储限制见 sp-kdbx 模块文档）
#[derive(uniffi::Record, Debug, Clone)]
pub struct FfiCustomField {
    pub name: String,
    pub value: String,
    pub protected: bool,
}

/// 条目草稿（写入面：add/edit 共用；与 FfiEntry 读回面分离）
#[derive(uniffi::Record, Debug, Clone)]
pub struct FfiEntryDraft {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub otp: Option<String>,
    pub custom_fields: Vec<FfiCustomField>,
    /// 标签分类（SP-F-108，S1b-2）
    pub tags: Vec<String>,
}

/// 条目全文完整读回（**仅 Rust 面，不经 UniFFI 导出**）。
/// 动机：FfiEntry 读回面不含 custom_fields（v0.1 FFI 限制）——桌面 GUI 编辑若以
/// FfiEntry 回写会清空自定义字段（design_docs/15 §9 同款已知限制）。本类型含
/// custom_fields，供 desktop GUI 编辑保真；不含 uniffi derive → Kotlin 绑定零漂移。
#[derive(Debug, Clone)]
pub struct FfiEntryFull {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub otp: Option<String>,
    pub custom_fields: Vec<FfiCustomField>,
    /// 标签分类（SP-F-108，S1b-2）
    pub tags: Vec<String>,
}

/// 统一错误面（映射 sp_core::SpError 子集；凭据语义不细化以防侧信道，SP-F-107）
#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum FfiError {
    #[error("credentials rejected (wrong password or missing keyfile)")]
    Credentials,
    #[error("not found: {msg}")]
    NotFound { msg: String },
    #[error("database format error: {msg}")]
    Format { msg: String },
    #[error("io error: {msg}")]
    Io { msg: String },
    #[error("invalid parameter: {msg}")]
    Parameter { msg: String },
}

impl From<SpError> for FfiError {
    fn from(e: SpError) -> Self {
        match e {
            SpError::Credentials => FfiError::Credentials,
            SpError::NotFound(s) => FfiError::NotFound { msg: s },
            SpError::Format(s) | SpError::Config(s) | SpError::Clipboard(s) => {
                FfiError::Format { msg: s }
            }
            SpError::Io(s) => FfiError::Io { msg: s },
            SpError::Parameter(s) => FfiError::Parameter { msg: s },
        }
    }
}

fn worker_unavailable() -> FfiError {
    FfiError::Format {
        msg: "vault worker unavailable (locked or dropped)".into(),
    }
}

fn ffi_entry(m: EntryModel) -> FfiEntry {
    FfiEntry {
        title: m.title,
        username: m.username,
        password: m.password.map(|p| p.as_str().to_string()),
        url: m.url,
        notes: m.notes,
        otp: m.otp,
        tags: m.tags,
    }
}

/// 完整读回映射（含 custom_fields；仅 Rust 面消费，design_docs/16 v1.2）
fn ffi_entry_full(m: EntryModel) -> FfiEntryFull {
    FfiEntryFull {
        title: m.title,
        username: m.username,
        password: m.password.map(|p| p.as_str().to_string()),
        url: m.url,
        notes: m.notes,
        otp: m.otp,
        custom_fields: m
            .custom_fields
            .into_iter()
            .map(|f| FfiCustomField {
                name: f.name,
                value: f.value.as_str().to_string(),
                protected: f.protected,
            })
            .collect(),
        tags: m.tags,
    }
}

fn draft_model(d: FfiEntryDraft) -> EntryModel {
    EntryModel {
        title: d.title,
        username: d.username,
        password: d.password.map(zeroize::Zeroizing::new),
        url: d.url,
        notes: d.notes,
        custom_fields: d
            .custom_fields
            .into_iter()
            .map(|f| sp_core::CustomField {
                name: f.name,
                value: zeroize::Zeroizing::new(f.value),
                protected: f.protected,
            })
            .collect(),
        otp: d.otp,
        tags: d.tags,
    }
}

// ---------- 函数面 ----------

fn factors(password: String, keyfile_path: Option<String>) -> CredentialFactors {
    CredentialFactors::new(
        SecureString::new(password),
        keyfile_path.map(std::path::PathBuf::from),
    )
}

/// 创建 KDBX4 库（语义 = sp_kdbx::create_db；一次性操作，不经 actor）
#[uniffi::export]
pub fn create_db(
    path: String,
    password: String,
    keyfile_path: Option<String>,
    params: FfiKdfParams,
) -> Result<(), FfiError> {
    sp_kdbx::create_db(
        std::path::Path::new(&path),
        &factors(password, keyfile_path),
        &params.into(),
    )?;
    Ok(())
}

/// 打开库并返回 Vault 句柄（KDF 在本次调用内完成，工作线程上执行）
#[uniffi::export]
pub fn open_db(
    path: String,
    password: String,
    keyfile_path: Option<String>,
) -> Result<Arc<Vault>, FfiError> {
    let (cmd_tx, cmd_rx) = channel::<VaultCmd>();
    let (resp_tx, resp_rx) = channel::<Result<(), FfiError>>();
    let f = factors(password, keyfile_path);
    std::thread::Builder::new()
        .name("sp-vault".into())
        .spawn(move || vault_worker(cmd_rx, path, f, resp_tx))
        .map_err(|e| FfiError::Io {
            msg: format!("spawn vault worker: {e}"),
        })?;
    let opened = resp_rx.recv().map_err(|_| FfiError::Format {
        msg: "vault worker died before opening".into(),
    })?;
    opened?;
    Ok(Arc::new(Vault { tx: cmd_tx }))
}

/// URL 匹配打分（SP-F-706/803 单源语义）：对条目 URL 列表按页面 URL 打分，
/// 返回按分数降序的条目索引（同分稳定）。page 或条目非法/不匹配则不出现在结果中。
#[uniffi::export]
pub fn rank_entry_urls(page_url: String, entry_urls: Vec<Option<String>>) -> Vec<u32> {
    let refs: Vec<Option<&str>> = entry_urls.iter().map(|o| o.as_deref()).collect();
    sp_core::match_url::rank(&page_url, &refs)
        .into_iter()
        .map(|i| i as u32)
        .collect()
}

/// 移动端 KDF 校准（DR-010 终态）：m=64MiB/p=4 固定，t∈[2,8] 实测校准至 target_ms。
/// 耗时数次 Argon2id 实测（每次 ~0.3–1s 量级），调用方应在后台线程执行。
#[uniffi::export]
pub fn calibrate_kdf(target_ms: u32) -> Result<FfiKdfParams, FfiError> {
    Ok(sp_core::calibration::calibrate_mobile(u64::from(target_ms))?.into())
}

// ---------- actor 工作线程 ----------

type Res<T> = Sender<Result<T, FfiError>>;

enum VaultCmd {
    FindEntry {
        title: String,
        resp: Res<FfiEntry>,
    },
    /// 完整读回（仅 Rust 面：FindEntryFull → FfiEntryFull，含 custom_fields）
    FindEntryFull {
        title: String,
        resp: Res<FfiEntryFull>,
    },
    ListSummaries {
        resp: Res<Vec<FfiEntrySummary>>,
    },
    TotpNow {
        title: String,
        resp: Res<String>,
    },
    /// 删除条目（仅 Rust 面：语义 = sp_kdbx::delete_entry）
    DeleteEntry {
        title: String,
        resp: Res<()>,
    },
    AddEntry {
        draft: FfiEntryDraft,
        resp: Res<()>,
    },
    EditEntry {
        title: String,
        draft: FfiEntryDraft,
        resp: Res<()>,
    },
    Save {
        path: String,
        factors: CredentialFactors,
        resp: Res<()>,
    },
    /// DR-012 A1：复用工作线程内驻留的凭据因子（open_db 时随 open 留存）
    SaveSession {
        path: String,
        resp: Res<()>,
    },
    AddAttachment {
        title: String,
        name: String,
        data: Vec<u8>,
        resp: Res<()>,
    },
    GetAttachment {
        title: String,
        name: String,
        resp: Res<Vec<u8>>,
    },
    Close {
        resp: Res<()>,
    },
}

fn respond<T>(resp: Res<T>, r: Result<T, FfiError>) {
    let _ = resp.send(r);
}

/// 工作线程主体：Database 全程封闭在本线程（满足 keepass_ng !Send 约束）。
/// DR-012 A1：凭据因子随 open 留驻本线程（Zeroizing 缓冲，锁库/线程退出即 drop 零化），
/// 供保存场景复用（save_session）；主密码内存驻留窗口=会话期，审查口径见 design_docs/14。
#[allow(clippy::too_many_lines)]
fn vault_worker(
    cmd_rx: std::sync::mpsc::Receiver<VaultCmd>,
    path: String,
    f: CredentialFactors,
    opened: Res<()>,
) {
    let mut db = match sp_kdbx::open_db(std::path::Path::new(&path), &f) {
        Ok(db) => {
            let _ = opened.send(Ok(()));
            db
        }
        Err(e) => {
            let _ = opened.send(Err(e.into()));
            return;
        }
    };
    let session_factors = f;
    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            VaultCmd::FindEntry { title, resp } => respond(
                resp,
                sp_kdbx::find_entry(&db, &title)
                    .map(ffi_entry)
                    .map_err(FfiError::from),
            ),
            VaultCmd::FindEntryFull { title, resp } => respond(
                resp,
                sp_kdbx::find_entry(&db, &title)
                    .map(ffi_entry_full)
                    .map_err(FfiError::from),
            ),
            VaultCmd::DeleteEntry { title, resp } => respond(
                resp,
                sp_kdbx::delete_entry(&mut db, &title).map_err(FfiError::from),
            ),
            VaultCmd::ListSummaries { resp } => respond(
                resp,
                Ok(sp_kdbx::list_entry_summaries(&db)
                    .into_iter()
                    .map(|s| FfiEntrySummary {
                        title: s.title,
                        username: s.username,
                        url: s.url,
                        has_otp: s.has_otp,
                        tags: s.tags,
                    })
                    .collect()),
            ),
            VaultCmd::TotpNow { title, resp } => respond(
                resp,
                sp_kdbx::entry_totp_now(&db, &title).map_err(FfiError::from),
            ),
            VaultCmd::AddEntry { draft, resp } => respond(
                resp,
                sp_kdbx::add_entry(&mut db, &draft_model(draft)).map_err(FfiError::from),
            ),
            VaultCmd::EditEntry { title, draft, resp } => respond(
                resp,
                sp_kdbx::edit_entry(&mut db, &title, &draft_model(draft)).map_err(FfiError::from),
            ),
            VaultCmd::Save {
                path,
                factors,
                resp,
            } => respond(
                resp,
                sp_kdbx::save_atomic(&db, std::path::Path::new(&path), &factors)
                    .map_err(FfiError::from),
            ),
            VaultCmd::SaveSession { path, resp } => respond(
                resp,
                sp_kdbx::save_atomic(&db, std::path::Path::new(&path), &session_factors)
                    .map_err(FfiError::from),
            ),
            VaultCmd::AddAttachment {
                title,
                name,
                data,
                resp,
            } => respond(
                resp,
                sp_kdbx::add_attachment(&mut db, &title, &name, &data).map_err(FfiError::from),
            ),
            VaultCmd::GetAttachment { title, name, resp } => respond(
                resp,
                sp_kdbx::get_attachment(&db, &title, &name).map_err(FfiError::from),
            ),
            VaultCmd::Close { resp } => {
                respond(resp, Ok(()));
                break; // db 在此 drop：凭据/派生材料随库释放（SP-F-802 锁库语义）
            }
        }
    }
    // cmd_tx 全部句柄释放（Vault 被 Kotlin GC 回收/Drop）→ 线程退出，Database drop
}

// ---------- 对象面 ----------

/// 已解锁库句柄：线程封闭 actor（Database 不跨线程，安全性由构造保证而非 unsafe）
#[derive(uniffi::Object, Debug)]
pub struct Vault {
    tx: Sender<VaultCmd>,
}

impl Drop for Vault {
    fn drop(&mut self) {
        // 断开 channel：工作线程在下一次 recv 失败后退出并 drop Database
    }
}

#[uniffi::export]
impl Vault {
    /// 按标题取回条目全文（语义 = sp_kdbx::find_entry）
    pub fn find_entry(&self, title: String) -> Result<FfiEntry, FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::FindEntry { title, resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 列出条目摘要（不含密码，语义 = sp_kdbx::list_entry_summaries）
    pub fn list_entry_summaries(&self) -> Result<Vec<FfiEntrySummary>, FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::ListSummaries { resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 生成条目当前 TOTP 码（语义 = sp_kdbx::entry_totp_now）
    pub fn entry_totp_now(&self, title: String) -> Result<String, FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::TotpNow { title, resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 新增条目（语义 = sp_kdbx::add_entry；校验规则同 sp-core）
    pub fn add_entry(&self, draft: FfiEntryDraft) -> Result<(), FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::AddEntry { draft, resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 编辑条目（旧快照进 history，语义 = sp_kdbx::edit_entry）
    pub fn edit_entry(&self, title: String, draft: FfiEntryDraft) -> Result<(), FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::EditEntry {
                title,
                draft,
                resp: tx,
            })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 原子保存（语义 = sp_kdbx::save_atomic；凭据由调用方再次提供，进程内传递不落盘）
    pub fn save(
        &self,
        path: String,
        password: String,
        keyfile_path: Option<String>,
    ) -> Result<(), FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::Save {
                path,
                factors: factors(password, keyfile_path),
                resp: tx,
            })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 会话凭据原子保存（DR-012 A1）：复用 open_db 时留驻工作线程的凭据因子，
    /// 调用方不再持有/传递主密码。库锁定（显式 lock 或空闲超时）后因子随
    /// actor drop 零化，本方法返回 Format("vault worker unavailable")。
    /// 主密码内存驻留窗口=会话期（design_docs/14 审查口径）。
    pub fn save_session(&self, path: String) -> Result<(), FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::SaveSession { path, resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 添加附件（语义 = sp_kdbx::add_attachment；保存需另调 save）
    pub fn add_attachment(
        &self,
        title: String,
        name: String,
        data: Vec<u8>,
    ) -> Result<(), FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::AddAttachment {
                title,
                name,
                data,
                resp: tx,
            })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 读取附件（语义 = sp_kdbx::get_attachment）
    pub fn get_attachment(&self, title: String, name: String) -> Result<Vec<u8>, FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::GetAttachment {
                title,
                name,
                resp: tx,
            })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 显式锁库（SP-F-802）：工作线程 drop Database（凭据随 drop 零化）并退出。
    /// 锁后任何操作返回 Format("vault worker unavailable")。
    pub fn lock(&self) -> Result<(), FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::Close { resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }
}

// ---------- Rust-only 面（不经 UniFFI 导出；desktop GUI 写路径专用） ----------
// 本块方法不出现在 Kotlin/Python 绑定中（无 #[uniffi::export]）——桌面以 Rust 依赖
// 方式使用 sp-kffi，直接调用；Android 侧不受影响，绑定漂移检查零变化。
impl Vault {
    /// 完整读回（含 custom_fields）：GUI 编辑保真前提（FfiEntry 读回面不含
    /// custom_fields，直接回写会清空——design_docs/15 §9 已知限制的桌面侧解法）。
    pub fn find_entry_full(&self, title: String) -> Result<FfiEntryFull, FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::FindEntryFull { title, resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }

    /// 删除条目（语义 = sp_kdbx::delete_entry；保存需另调 save/save_session）
    pub fn delete_entry(&self, title: String) -> Result<(), FfiError> {
        let (tx, rx) = channel();
        self.tx
            .send(VaultCmd::DeleteEntry { title, resp: tx })
            .map_err(|_| worker_unavailable())?;
        rx.recv().map_err(|_| worker_unavailable())?
    }
}

uniffi::setup_scaffolding!();
