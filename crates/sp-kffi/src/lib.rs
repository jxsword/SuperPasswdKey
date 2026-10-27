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
    fn default() -> Self {
        KdfParams::default().into()
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
}

/// 条目摘要（不含密码/TOTP，对齐 sp_kdbx::EntrySummary / SP-F-706 语义）
#[derive(uniffi::Record, Debug, Clone)]
pub struct FfiEntrySummary {
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub has_otp: bool,
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

// ---------- actor 工作线程 ----------

type Res<T> = Sender<Result<T, FfiError>>;

enum VaultCmd {
    FindEntry {
        title: String,
        resp: Res<FfiEntry>,
    },
    ListSummaries {
        resp: Res<Vec<FfiEntrySummary>>,
    },
    TotpNow {
        title: String,
        resp: Res<String>,
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
/// 凭据因子仅在 open/save 调用栈内存在，用后即 drop（zeroize）。
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
    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            VaultCmd::FindEntry { title, resp } => respond(
                resp,
                sp_kdbx::find_entry(&db, &title)
                    .map(ffi_entry)
                    .map_err(FfiError::from),
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

uniffi::setup_scaffolding!();
