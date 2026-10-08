// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 磁盘持久化 fallback 队列（deferred-capabilities C4 + R-inklog-002）。
//!
//! ERROR/FATAL 兜底缓冲（内存 100 条 LRU）溢出/淘汰的记录落盘 journal，
//! 进程启动时经 [`FallbackJournal::replay`] 重放一次后清空——关键日志跨进程
//! 不丢。落盘不做 fsync：承诺"进程崩溃后可重放"（OS page cache 级），断电
//! 窗口与 file sink 一致。
//!
//! 可选能力（默认关闭，行为与既往一致）：
//! - **AES-256-GCM 加密**：[`FallbackJournal::with_crypto`]，key 来源
//!   env > 配置文件（key 文件权限 0600），magic+版本头，replay 新旧格式
//!   自适应；key 缺失/无效构造期显性失败，禁止静默明文；
//! - **推送端口**：[`JournalPushSender`]（无网络环境可注入 mock），
//!   [`TcpJournalPusher`] 复用 net-sink 断线缓冲范式并预留 TLS 扩展点。

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use parking_lot::{Condvar, Mutex};
use rand::Rng;
use zeroize::Zeroizing;

use crate::InklogError;
use crate::LogRecord;
use crate::support::io::sink::encryption::{
    KeyMaterial, resolve_key_material, validate_key_entropy,
};

/// 默认 journal 容量上限：10 MiB。
pub const DEFAULT_JOURNAL_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// journal 加密密钥的默认环境变量名（未配置 `key_env` 时生效）。
pub const DEFAULT_JOURNAL_KEY_ENV: &str = "INKLOG_JOURNAL_KEY";

/// 加密 journal 文件头 magic（8 字节）。
pub const JOURNAL_MAGIC: [u8; 8] = *b"INKJRN1\0";

/// 加密 journal 格式版本。
pub const JOURNAL_FORMAT_VERSION: u16 = 1;

/// 算法标识：AES-256-GCM。
pub const JOURNAL_ALGO_AES256_GCM: u16 = 1;

/// 文件头长度 = magic(8) + version(2) + algo(2) + salt(16)。
pub const JOURNAL_HEADER_LEN: usize = 28;

/// 段 nonce 长度（96-bit；唯一性策略见 `spill_encrypted`）。
const NONCE_LEN: usize = 12;

/// 段定长前缀 = nonce(12) + ciphertext 长度 u32le(4)。
const SEGMENT_PREFIX_LEN: usize = NONCE_LEN + 4;

/// journal 加密配置：key 来源**env 优先，其次 key 文件**（unix 权限必须
/// 0600）。构造时解析并校验——缺失/无效即显性失败，禁止静默明文落盘。
#[derive(Debug, Clone, Default)]
pub struct JournalCryptoConfig {
    /// 密钥环境变量名；`None` 用 [`DEFAULT_JOURNAL_KEY_ENV`]。
    pub key_env: Option<String>,
    /// 密钥文件路径（env 未设置时读取）。
    pub key_file: Option<PathBuf>,
}

/// journal 推送端口：把 spill 的记录同步交给实现方投递。
///
/// 实现方自行决定传输、缓冲与重试语义（参考实现 [`TcpJournalPusher`] 复用
/// net-sink 断线缓冲范式）；推送失败**不得**反压落盘路径——落盘失败才返回
/// `false`，推送失败只计数。
pub trait JournalPushSender: Send + Sync {
    /// 推送一条序列化记录（无换行符，传输层自行加帧）。
    fn push(&self, payload: &[u8]) -> Result<(), InklogError>;

    /// 尽力补发内部缓冲（进程退出前调用）；默认 no-op。
    fn flush_pending(&self) {}
}

/// 传输安全扩展点（预留）。
///
/// 当前仅明文 TCP；TLS（rustls 客户端，对齐 net-sink 的 `TlsClientConfig`
/// 范式）作为后续变体加入——`non_exhaustive` 保证加变体不破坏 semver。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportSecurity {
    /// 明文 TCP（默认）。
    #[default]
    None,
}

/// TCP journal 推送配置。
#[derive(Debug, Clone)]
pub struct TcpJournalPusherConfig {
    /// 目标地址（`host:port`）。
    pub addr: String,
    /// 连接超时（默认 3s）。
    pub connect_timeout: Duration,
    /// 单次写超时（默认 3s）：对端不排空时写阻塞到此即视为连接死亡，
    /// 走既有弃连 + 重连 + 缓冲路径；网络停滞由专职投递线程在 `state` 锁外
    /// 消化，调用方 push 仅 O(1) 入队，flush/退出路径有界（见 flush_pending）。
    pub write_timeout: Duration,
    /// 断线缓冲容量（条数，满则丢最旧；默认 10000）。
    pub buffer_capacity: usize,
    /// 传输安全（预留扩展点，当前仅明文）。
    pub transport_security: TransportSecurity,
}

impl Default for TcpJournalPusherConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:5170".to_string(),
            connect_timeout: Duration::from_secs(3),
            write_timeout: Duration::from_secs(3),
            buffer_capacity: 10_000,
            transport_security: TransportSecurity::None,
        }
    }
}

struct PusherState {
    buffer: VecDeque<Vec<u8>>,
}

/// 投递串行化状态：连接句柄与单轮投递权（投递线程与 flush_pending 共用）。
struct DeliverState {
    stream: Option<std::net::TcpStream>,
}

/// 投递线程与调用方共享的状态。
///
/// 锁模型：`state` 只保护断线缓冲，持锁仅 O(1) 换出/回填批量；连接与网络
/// IO（建连、半开探测、按序补发、写超时）在 `deliver` 锁内、`state` 锁外
/// 执行——push 关键路径不被网络停滞阻塞，投递线程退避期限时等待也不持
/// `state` 锁忙自旋。
struct PusherShared {
    config: TcpJournalPusherConfig,
    state: Mutex<PusherState>,
    /// 连接句柄 + 单轮投递权（投递线程与 flush 串行化，保证帧序）
    deliver: Mutex<DeliverState>,
    /// 新条目 / 退避到期 / shutdown 唤醒投递线程
    signal: Condvar,
    shutdown: AtomicBool,
    /// 连续 connect 失败次数（指数退避依据）
    consecutive_failures: AtomicU32,
    /// 退避截止时间（unix 毫秒）；期间投递线程限时等待
    backoff_until_ms: AtomicU64,
    /// 缓冲满被丢弃的总条数（可观测）
    dropped_total: AtomicU64,
}

impl PusherShared {
    /// 指数退避间隔：min(2^n × 100ms, 30s)。n 为连续失败次数（0 起）。
    fn backoff_delay_ms(failures: u32) -> u64 {
        let step = 100u64.saturating_mul(1u64 << failures.min(31));
        step.min(30_000)
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn connect_allowed(&self) -> bool {
        Self::now_ms() >= self.backoff_until_ms.load(Ordering::Acquire)
    }

    fn backoff_remaining(&self) -> Duration {
        let ms = self
            .backoff_until_ms
            .load(Ordering::Acquire)
            .saturating_sub(Self::now_ms())
            .min(30_000);
        Duration::from_millis(ms)
    }

    fn note_connect_result(&self, ok: bool) {
        if ok {
            self.consecutive_failures.store(0, Ordering::Release);
            self.backoff_until_ms.store(0, Ordering::Release);
        } else {
            let n = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
            let delay = Self::backoff_delay_ms(n);
            self.backoff_until_ms
                .store(Self::now_ms() + delay, Ordering::Release);
        }
    }

    /// 批量头部回填（投递失败剩余）：晚于批量到达的新条目排在其后，帧序
    /// 不乱。空到非空的转换必唤醒投递线程，避免其在空缓冲等待中丢唤醒。
    fn requeue_front(&self, batch: VecDeque<Vec<u8>>) {
        if batch.is_empty() {
            return;
        }
        let mut st = self.state.lock();
        for line in batch.into_iter().rev() {
            st.buffer.push_front(line);
        }
        self.signal.notify_one();
    }

    /// 单轮投递：`deliver` 锁内换出批量 → 网络 IO（不持 `state` 锁）→ 剩余回填。
    ///
    /// `state` 锁仅在换出/回填瞬间持有，网络 IO（建连、半开探测、按序补发、
    /// 写超时）全部不持 `state` 锁——push 与观测调用不被网络停滞阻塞。
    /// `deliver` 锁串行化投递线程与 flush_pending 的单轮投递（连接句柄随之
    /// 独占），同一条连接不会被交叉写破坏帧序。返回 `false` = 本轮未投递
    /// （退避中 / 建连失败 / 连接死亡），批量已回队待下次投递。
    fn deliver_one_round(&self) -> bool {
        let mut deliver = self.deliver.lock();
        let batch = {
            let mut st = self.state.lock();
            if st.buffer.is_empty() {
                return true;
            }
            std::mem::take(&mut st.buffer)
        };
        if deliver.stream.is_none() {
            if !self.connect_allowed() {
                drop(deliver);
                self.requeue_front(batch);
                return false;
            }
            match TcpJournalPusher::connect(&self.config) {
                Ok(new_stream) => {
                    deliver.stream = Some(new_stream);
                    self.note_connect_result(true);
                }
                Err(_) => {
                    self.note_connect_result(false);
                    drop(deliver);
                    self.requeue_front(batch);
                    return false;
                }
            }
        }
        let Some(stream) = deliver.stream.as_mut() else {
            unreachable!("stream retained or established above");
        };
        if TcpJournalPusher::connection_dead(stream) {
            deliver.stream = None;
            drop(deliver);
            self.requeue_front(batch);
            return false;
        }
        let mut remainder = batch;
        if !TcpJournalPusher::drain_buffer(stream, &mut remainder) {
            // 写失败/写超时：弃用连接，剩余缓冲回队（下次投递重连补发）
            deliver.stream = None;
        }
        drop(deliver);
        if !remainder.is_empty() {
            self.requeue_front(remainder);
        }
        true
    }
}

/// TCP journal 推送器：复用 net-sink `TcpSink`（`net-sink` feature，其文档链接在该 feature 未启用时不可解析，故不用 intra-doc link）
/// 的断线缓冲范式——有界 FIFO 缓冲（满则丢最旧）、指数退避自动重连、
/// 连接恢复后按序补发。`push` 永不因网络失败向调用方报错（尽力而为），
/// 丢弃量经 [`TcpJournalPusher::dropped_total`] 可观测。
///
/// 线程模型：构造即启动专职投递线程——连接、半开探测、补发、写超时重连
/// 全部在该线程执行且不持 `state` 锁（批量换出后在锁外投递）；`push`
/// 关键路径只有锁内入队 + 唤醒（O(1)），网络停滞不再传导为调用方停顿。
/// 写超时（`write_timeout`）触发即弃用连接并保留缓冲待重连补发。
///
/// TLS 说明：接收端服务与 TLS 传输属后续扩展（见 [`TransportSecurity`]）；
/// 当前为明文 NDJSON 出站，部署在可信网段或由隧道层提供传输安全。
pub struct TcpJournalPusher {
    shared: Arc<PusherShared>,
    /// 投递线程句柄（Drop 时 join）
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl TcpJournalPusher {
    /// 创建推送器并启动投递线程；地址仅做 `host:port` 语法校验——瞬时
    /// DNS 故障不应让「尽力而为」推送端口以 ConfigError 终止整个构建，
    /// 解析在投递线程的 connect 处进行。
    pub fn new(config: TcpJournalPusherConfig) -> Result<Self, InklogError> {
        Self::validate_addr_syntax(&config.addr)?;
        let shared = Arc::new(PusherShared {
            config,
            state: Mutex::new(PusherState {
                buffer: VecDeque::new(),
            }),
            deliver: Mutex::new(DeliverState { stream: None }),
            signal: Condvar::new(),
            shutdown: AtomicBool::new(false),
            consecutive_failures: AtomicU32::new(0),
            backoff_until_ms: AtomicU64::new(0),
            dropped_total: AtomicU64::new(0),
        });
        let worker_shared = Arc::clone(&shared);
        let handle = thread::Builder::new()
            .name("inklog-journal-push".to_string())
            .spawn(move || Self::worker_loop(worker_shared))
            .map_err(|e| {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("err", e);
                InklogError::ChannelError(crate::i18n::tr_args("journal-push-spawn-failed", args))
            })?;
        Ok(Self {
            shared,
            worker: Mutex::new(Some(handle)),
        })
    }

    /// `host:port` 语法校验（host 非空、port 为 u16；IPv6 字面量 `[::1]:p`）。
    fn validate_addr_syntax(addr: &str) -> Result<(), InklogError> {
        let valid = match addr.rsplit_once(':') {
            Some((host, port)) => {
                let host = host.trim_start_matches('[').trim_end_matches(']');
                !host.is_empty() && port.parse::<u16>().is_ok()
            }
            None => false,
        };
        if !valid {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("addr", addr.to_string());
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-journal_push_invalid_addr",
                args,
            )));
        }
        Ok(())
    }

    fn connect(config: &TcpJournalPusherConfig) -> std::io::Result<std::net::TcpStream> {
        use std::net::ToSocketAddrs as _;
        let addr = config.addr.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                crate::i18n::tr("journal-push-empty-addr"),
            )
        })?;
        let stream = std::net::TcpStream::connect_timeout(&addr, config.connect_timeout)?;
        stream.set_nodelay(true).ok();
        // 写超时：对端接受连接但不排空时，写阻塞到此即出错（视为连接死亡）
        stream.set_write_timeout(Some(config.write_timeout)).ok();
        Ok(stream)
    }

    /// 写前健康探测：对端已 FIN/RST（半开连接）时返回 true——避免首笔写
    /// 静默落入死连接（TCP 写后知错的固有窗口）。与 net-sink TcpSink
    /// 同一范式（明文流非阻塞 peek 检出）。
    fn connection_dead(stream: &mut std::net::TcpStream) -> bool {
        let _ = stream.set_nonblocking(true);
        let mut probe = [0u8; 1];
        let verdict = match stream.peek(&mut probe) {
            // 对端关闭：读到 EOF
            Ok(0) => true,
            // 仍有数据/暂无数据：视为存活（单向日志流不读应用数据）
            Ok(_) => false,
            Err(e) => matches!(
                e.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::NotConnected
            ),
        };
        // 阻塞模式恢复失败则弃用连接：非阻塞残留会让后续写入全部 WouldBlock
        if stream.set_nonblocking(false).is_err() {
            return true;
        }
        verdict
    }

    /// 入队断线缓冲（满则丢最旧，容量 0 视为立即丢弃）。
    fn enqueue(buffer: &mut VecDeque<Vec<u8>>, capacity: usize, line: Vec<u8>) -> u64 {
        if capacity == 0 {
            return 1;
        }
        let mut dropped = 0u64;
        if buffer.len() >= capacity {
            buffer.pop_front();
            dropped += 1;
        }
        buffer.push_back(line);
        dropped
    }

    /// 尽力按序补发缓冲；单笔写失败（含写超时）即停止并报告未清空
    /// （剩余保留，弃连后下次投递重连补发）。
    fn drain_buffer(stream: &mut std::net::TcpStream, buffer: &mut VecDeque<Vec<u8>>) -> bool {
        while let Some(front) = buffer.front() {
            if stream.write_all(front).is_err() {
                return false;
            }
            buffer.pop_front();
        }
        stream.flush().is_ok()
    }

    /// 投递线程主体：有积压时换出批量做单轮投递；缓冲空 / 退避窗口在
    /// `state` 锁的 Condvar 上限时等待（等待期间锁释放，不忙自旋），
    /// 新条目 / 退避到期 / shutdown 提前唤醒。
    fn worker_loop(shared: Arc<PusherShared>) {
        loop {
            if shared.shutdown.load(Ordering::Acquire) {
                return;
            }
            {
                let mut st = shared.state.lock();
                loop {
                    if shared.shutdown.load(Ordering::Acquire) {
                        return;
                    }
                    if !st.buffer.is_empty() {
                        break;
                    }
                    if shared.connect_allowed() {
                        shared.signal.wait(&mut st);
                    } else {
                        // 退避窗口：限时等待，到期或新条目唤醒后重判
                        let _ = shared.signal.wait_for(&mut st, shared.backoff_remaining());
                    }
                }
            }
            if !shared.deliver_one_round() {
                // 本轮因退避/建连失败未投递（批量已回队）：限时等待后重试，
                // 不持锁忙自旋；新条目唤醒提前重判
                let mut st = shared.state.lock();
                if !shared.connect_allowed() {
                    let _ = shared.signal.wait_for(&mut st, shared.backoff_remaining());
                }
            }
        }
    }

    /// 当前断线缓冲条数。
    pub fn buffered_len(&self) -> usize {
        self.shared.state.lock().buffer.len()
    }

    /// 缓冲满被丢弃的累计条数。
    pub fn dropped_total(&self) -> u64 {
        self.shared.dropped_total.load(Ordering::Relaxed)
    }

    /// 尽力补发断线缓冲（进程退出前调用）：与投递线程共用
    /// `PusherShared::deliver_one_round`（`deliver` 锁串行化，帧序一致）。
    /// 预算 = connect + write 超时——对端不可达/不排空时有限时间返回；
    /// 网络 IO 不持 `state` 锁，也不与投递线程死锁。
    pub fn flush_pending(&self) {
        let deadline = std::time::Instant::now()
            + self.shared.config.connect_timeout
            + self.shared.config.write_timeout;
        while std::time::Instant::now() < deadline {
            if !self.shared.deliver_one_round() {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            if self.shared.state.lock().buffer.is_empty() {
                return;
            }
        }
    }
}

impl JournalPushSender for TcpJournalPusher {
    fn push(&self, payload: &[u8]) -> Result<(), InklogError> {
        let mut line = payload.to_vec();
        line.push(b'\n');

        // 关键路径只做锁内入队（O(1)）+ 唤醒投递线程：连接、半开探测、
        // 补发与重连全部在专职线程执行——此前同步 push 在对端不排空时
        // 会无限冻结调用线程，且锁内网络 IO 串行化所有 spill 调用方。
        let dropped = {
            let mut st = self.shared.state.lock();
            Self::enqueue(&mut st.buffer, self.shared.config.buffer_capacity, line)
        };
        self.shared
            .dropped_total
            .fetch_add(dropped, Ordering::Relaxed);
        self.shared.signal.notify_one();
        Ok(())
    }

    fn flush_pending(&self) {
        TcpJournalPusher::flush_pending(self);
    }
}

impl Drop for TcpJournalPusher {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        self.shared.signal.notify_all();
        if let Some(handle) = self.worker.lock().take() {
            let _ = handle.join();
        }
    }
}

/// 已解析的 journal 加密状态（构造期 key 缺失/无效不可能到达此处）。
///
/// (salt, 派生密钥, cipher 实例) 缓存：同一文件头盐免重复 PBKDF2（600k
/// 迭代/次）与 cipher 构建。
type DerivedKeyCache = Mutex<Option<([u8; 16], Zeroizing<[u8; 32]>, Aes256Gcm)>>;

struct JournalCrypto {
    material: KeyMaterial,
    derived: DerivedKeyCache,
}

impl JournalCrypto {
    /// 按文件头盐取派生密钥与 cipher 实例（缓存命中零派生零构建开销）。
    /// 密钥恒为 32 字节，cipher 构建不可能失败。
    fn cipher_for_salt(&self, salt: &[u8; 16]) -> (Zeroizing<[u8; 32]>, Aes256Gcm) {
        let mut cache = self.derived.lock();
        if let Some((cached_salt, key, cipher)) = cache.as_ref()
            && cached_salt == salt
        {
            return (key.clone(), cipher.clone());
        }
        let key = self.material.derive_with_salt(salt);
        let cipher = Aes256Gcm::new_from_slice(key.as_slice()).expect("32-byte AES-256 key");
        *cache = Some((*salt, key.clone(), cipher.clone()));
        (key, cipher)
    }
}

/// 持久化 fallback 日志：明文模式为 JSONL（逐行一条 [`LogRecord`]，历史
/// 格式不变）；加密模式为 magic+版本头 + AES-256-GCM 逐段密文（段 =
/// nonce(12) + 长度(4) + ciphertext，明文为一行 JSON）。
///
/// **nonce 唯一性策略（实现前固化）**：随机 96-bit/段，CSPRNG
/// （`rand::rng()`，OS 定期重播种）生成；同 key 跨段、跨文件不重复由
/// 单测断言（96-bit 随机空间下生日碰撞概率可忽略）。
///
/// **显性失败约束**：[`FallbackJournal::with_crypto`] 在 key 缺失/无效时
/// 返回 `Err`——加密模式不存在"静默降级明文"路径。
pub struct FallbackJournal {
    path: PathBuf,
    max_bytes: u64,
    /// `Some` = 加密模式（密钥材料已在构造期解析）
    crypto: Option<JournalCrypto>,
    /// 可选推送端口（spill 同步投递，失败只计数不反压）
    sender: Option<Arc<dyn JournalPushSender>>,
    pushed_count: AtomicU64,
    push_failures: AtomicU64,
    /// 加密写路径互斥（头创建/容量截断/追加剧内原子；跨进程由部署层保证单写者）
    write_lock: Mutex<()>,
}

impl FallbackJournal {
    /// 以默认容量（10 MiB）打开明文 journal。
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self::with_limit(path, DEFAULT_JOURNAL_MAX_BYTES)
    }

    /// 以指定容量打开明文 journal。
    pub fn with_limit(path: impl Into<PathBuf>, max_bytes: u64) -> Self {
        Self {
            path: path.into(),
            max_bytes: max_bytes.max(1),
            crypto: None,
            sender: None,
            pushed_count: AtomicU64::new(0),
            push_failures: AtomicU64::new(0),
            write_lock: Mutex::new(()),
        }
    }

    /// 以加密模式打开 journal（R-inklog-002）。
    ///
    /// 文件格式：`JOURNAL_MAGIC`(8) + version(2, le) + algo(2, le) +
    /// salt(16) 头 + 逐段 `nonce(12) + ct_len(4, le) + AES-256-GCM 密文`。
    /// 密码模式密钥经 PBKDF2(密码, 头部盐) 确定性派生（盐随头存储，
    /// 解密方按头重导出）；原始/Base64 密钥直接使用并做熵校验。
    ///
    /// # Errors
    ///
    /// key 缺失（env 未设置且无 key 文件）、格式无效（Base64 长度不符 /
    /// 密码过短）或原始密钥熵不足时返回 `Err`——**显性失败，不落明文**。
    pub fn with_crypto(
        path: impl Into<PathBuf>,
        max_bytes: u64,
        config: JournalCryptoConfig,
    ) -> Result<Self, InklogError> {
        let env_var = config
            .key_env
            .unwrap_or_else(|| DEFAULT_JOURNAL_KEY_ENV.to_string());
        let material = resolve_key_material(&env_var, config.key_file.as_deref())?;
        // 纵深防御：直接用作密钥的原始字节须过熵校验（拒绝全零等弱密钥），
        // 与 FileSink 同一实现
        if let KeyMaterial::Raw(key) = &material {
            validate_key_entropy(key.as_slice())?;
        }
        Ok(Self {
            path: path.into(),
            max_bytes: max_bytes.max(1),
            crypto: Some(JournalCrypto {
                material,
                derived: Mutex::new(None),
            }),
            sender: None,
            pushed_count: AtomicU64::new(0),
            push_failures: AtomicU64::new(0),
            write_lock: Mutex::new(()),
        })
    }

    /// 挂载推送端口（builder 风格）。
    pub fn with_sender(mut self, sender: Arc<dyn JournalPushSender>) -> Self {
        self.sender = Some(sender);
        self
    }

    /// 累计推送成功条数。
    pub fn pushed_count(&self) -> u64 {
        self.pushed_count.load(Ordering::Relaxed)
    }

    /// 累计推送失败条数（失败不反压落盘，仅计数）。
    pub fn push_failure_count(&self) -> u64 {
        self.push_failures.load(Ordering::Relaxed)
    }

    /// 尽力补发推送端口缓冲（进程退出路径调用）；未挂载端口时为 no-op。
    pub fn flush_pending(&self) {
        if let Some(sender) = self.sender.as_ref() {
            sender.flush_pending();
        }
    }

    /// 追加一条记录。超过容量上限时截断头部（丢最旧）后再追加。
    ///
    /// 返回 `false` 表示 IO/加密失败（调用方静默计数，不得反压主链路）。
    /// 推送端口失败不影响返回值。
    pub fn spill(&self, record: &LogRecord) -> bool {
        let Ok(json) = serde_json::to_string(record) else {
            return false;
        };
        let mut line = json.into_bytes();
        line.push(b'\n');
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
            && std::fs::create_dir_all(parent).is_err()
        {
            return false;
        }
        let written = match self.crypto.as_ref() {
            Some(crypto) => {
                let _guard = self.write_lock.lock();
                self.spill_encrypted(crypto, &line)
            }
            None => self.spill_plain(&line),
        };
        if !written {
            return false;
        }
        // line 尾部换行外的部分即原始 JSON（零拷贝切片）
        self.push_to_sender(&line[..line.len() - 1]);
        true
    }

    /// 重放全部记录并清空 journal（原子语义：读取后截断）。
    ///
    /// 格式自适应：`JOURNAL_MAGIC` 头识别加密格式（逐段解密），否则按历史
    /// 明文 JSONL 逐行解析——旧明文文件可被启用加密的新版实例重放。
    /// 损坏行/不可解密段跳过并计数，不中断重放。返回 `(记录, 跳过数)`。
    ///
    /// **不可解密保留**：加密文件存在任何不可解段（错误密钥、中途换钥、
    /// 版本不支持、明文实例配置回滚）时 journal 文件原样保留，skipped 计数
    /// 显性上报——不可解段可能随密钥恢复/排查事后可解，字节销毁不可逆。
    /// 仅全部段成功重放（skipped == 0）才清空；部分成功场景下已重放记录在
    /// 下次 replay 重复上送（at-least-once），由调用方幂等消费。
    pub fn replay(&self) -> (Vec<LogRecord>, u64) {
        let Ok(data) = std::fs::read(&self.path) else {
            return (Vec::new(), 0);
        };
        if data.starts_with(&JOURNAL_MAGIC) {
            let (records, skipped) = self.replay_encrypted(&data);
            if skipped > 0 {
                tracing::warn!(
                    skipped,
                    replayed = records.len(),
                    path = %self.path.display(),
                    "{}",
                    crate::i18n::tr("journal-undecryptable-preserved")
                );
                return (records, skipped);
            }
            // 清空：重放一次后 journal 归零（加密模式下次 spill 重建头）
            let _ = std::fs::write(&self.path, b"");
            return (records, skipped);
        }
        // 历史明文 JSONL；无效 UTF-8 经 lossy 转换按损坏行计数
        //（此前非 UTF-8 文件会永久卡在"重放为空且不清空"，lossy 修复）
        let mut records = Vec::new();
        let mut skipped = 0u64;
        for line in String::from_utf8_lossy(&data).lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<LogRecord>(line) {
                Ok(record) => records.push(record),
                Err(_) => skipped += 1,
            }
        }
        let _ = std::fs::write(&self.path, b"");
        (records, skipped)
    }

    fn spill_plain(&self, line: &[u8]) -> bool {
        // 配置回滚护栏：明文实例不得向加密 journal 追加（会破坏段定长结构，
        // 且明文字节混入加密文件）。显性拒绝，沿用落盘失败语义（调用方计数）。
        if self.current_size() > 0 && self.file_starts_with_magic() {
            tracing::error!(
                path = %self.path.display(),
                "{}",
                crate::i18n::tr("journal-plaintext-append-refused")
            );
            return false;
        }
        if self.current_size() + line.len() as u64 > self.max_bytes {
            self.truncate_head_for(line.len() as u64);
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .and_then(|mut f| f.write_all(line))
            .is_ok()
    }

    /// 文件以 journal 加密 magic 开头（仅非空文件调用；O(1) 读 8 字节）。
    fn file_starts_with_magic(&self) -> bool {
        use std::io::Read as _;
        let Ok(mut f) = std::fs::File::open(&self.path) else {
            return false;
        };
        let mut magic = [0u8; 8];
        f.read_exact(&mut magic).is_ok() && magic == JOURNAL_MAGIC
    }

    /// 加密追加一段；文件头缺失时创建（新随机盐）。
    fn spill_encrypted(&self, crypto: &JournalCrypto, line: &[u8]) -> bool {
        let salt = match self.ensure_encrypted_header() {
            Ok(salt) => salt,
            Err(e) => {
                tracing::error!(error = %e, path = %self.path.display(),
                    "{}", crate::i18n::tr("journal-encrypted-spill-failed"));
                return false;
            }
        };
        let (_, cipher) = crypto.cipher_for_salt(&salt);

        // nonce 唯一性策略（固化）：随机 96-bit/段，CSPRNG 生成
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = aes_gcm::Nonce::from(nonce_bytes);

        let Ok(ciphertext) = cipher.encrypt(&nonce, line) else {
            tracing::error!(path = %self.path.display(),
                "{}", crate::i18n::tr("journal-aesgcm-encrypt-failed"));
            return false;
        };

        let mut segment = Vec::with_capacity(SEGMENT_PREFIX_LEN + ciphertext.len());
        segment.extend_from_slice(&nonce_bytes);
        segment.extend_from_slice(&(ciphertext.len() as u32).to_le_bytes());
        segment.extend_from_slice(&ciphertext);

        if self.current_size() + segment.len() as u64 > self.max_bytes {
            self.truncate_head_encrypted(segment.len() as u64);
        }
        open_journal_file(&self.path, true)
            .and_then(|mut f| f.write_all(&segment))
            .is_ok()
    }

    /// 读取或创建加密文件头；格式冲突（明文/异版文件）显性报错。
    ///
    /// 只读头 [`JOURNAL_HEADER_LEN`] 字节取盐（O(1)）——整文件读取会让每次
    /// spill 变成 O(文件大小)（容量上限 10 MiB），临界路径无谓放大 IO。
    fn ensure_encrypted_header(&self) -> Result<[u8; 16], InklogError> {
        use std::io::Read as _;
        let mut head = [0u8; JOURNAL_HEADER_LEN];
        let head_len = (|| -> std::io::Result<usize> {
            let mut f = std::fs::File::open(&self.path)?;
            let mut n = 0usize;
            loop {
                match f.read(&mut head[n..]) {
                    Ok(0) => break,
                    Ok(k) => {
                        n += k;
                        if n == JOURNAL_HEADER_LEN {
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
            Ok(n)
        })();
        let new_header = |path: &std::path::Path| -> Result<[u8; 16], InklogError> {
            let salt: [u8; 16] = rand::random();
            let mut header = Vec::with_capacity(JOURNAL_HEADER_LEN);
            header.extend_from_slice(&JOURNAL_MAGIC);
            header.extend_from_slice(&JOURNAL_FORMAT_VERSION.to_le_bytes());
            header.extend_from_slice(&JOURNAL_ALGO_AES256_GCM.to_le_bytes());
            header.extend_from_slice(&salt);
            open_journal_file(path, false)
                .and_then(|mut f| f.write_all(&header))
                .map_err(InklogError::IoError)?;
            Ok(salt)
        };
        match head_len {
            Ok(JOURNAL_HEADER_LEN) if head[..8] == JOURNAL_MAGIC => {
                let version = u16::from_le_bytes(head[8..10].try_into().expect("2 bytes"));
                let algo = u16::from_le_bytes(head[10..12].try_into().expect("2 bytes"));
                if version != JOURNAL_FORMAT_VERSION || algo != JOURNAL_ALGO_AES256_GCM {
                    return Err(InklogError::RuntimeError(crate::i18n::tr_args(
                        "config-journal_header_conflict",
                        header_conflict_args(&self.path),
                    )));
                }
                Ok(head[12..JOURNAL_HEADER_LEN].try_into().expect("16 bytes"))
            }
            // 文件缺失或为空：新建头（新随机盐，unix 0600）
            Ok(0) => new_header(&self.path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => new_header(&self.path),
            // 残缺头 / 明文内容：格式冲突
            _ => Err(InklogError::RuntimeError(crate::i18n::tr_args(
                "config-journal_header_conflict",
                header_conflict_args(&self.path),
            ))),
        }
    }

    /// 加密模式头部截断：走段边界丢最旧，保留头与其余段（tmp + rename 原子替换）。
    fn truncate_head_encrypted(&self, needed: u64) {
        let Ok(data) = std::fs::read(&self.path) else {
            return;
        };
        if data.len() <= JOURNAL_HEADER_LEN {
            return;
        }
        let mut segment_starts = Vec::new();
        let mut tail_end = JOURNAL_HEADER_LEN;
        let mut off = JOURNAL_HEADER_LEN;
        while off + SEGMENT_PREFIX_LEN <= data.len() {
            let ct_len = u32::from_le_bytes(
                data[off + NONCE_LEN..off + SEGMENT_PREFIX_LEN]
                    .try_into()
                    .expect("4 bytes"),
            ) as usize;
            let end = off + SEGMENT_PREFIX_LEN + ct_len;
            if end > data.len() {
                break;
            }
            segment_starts.push(off);
            tail_end = end;
            off = end;
        }
        let mut freed = 0u64;
        let mut keep_idx = 0usize;
        while keep_idx < segment_starts.len() && freed < needed {
            let start = segment_starts[keep_idx];
            let end = segment_starts
                .get(keep_idx + 1)
                .copied()
                .unwrap_or(tail_end);
            freed += (end - start) as u64;
            keep_idx += 1;
        }
        let body_start = segment_starts.get(keep_idx).copied().unwrap_or(tail_end);
        let mut body = Vec::with_capacity(JOURNAL_HEADER_LEN + data.len() - body_start);
        body.extend_from_slice(&data[..JOURNAL_HEADER_LEN]);
        body.extend_from_slice(&data[body_start..]);
        let tmp = self.path.with_extension("journal.tmp");
        // tmp 与兜底重写同为 0600：rename 后加密文件权限不得回退 0644
        if open_journal_file(&tmp, false)
            .and_then(|mut f| f.write_all(&body))
            .and_then(|_| std::fs::rename(&tmp, &self.path))
            .is_err()
        {
            // 截断失败：退回首部保全（宁可丢段也不超限增长）
            if let Ok(mut f) = open_journal_file(&self.path, false) {
                let _ = f.write_all(&data[..JOURNAL_HEADER_LEN]);
            }
        }
    }

    fn replay_encrypted(&self, data: &[u8]) -> (Vec<LogRecord>, u64) {
        if data.len() < JOURNAL_HEADER_LEN {
            return (Vec::new(), 0);
        }
        let Some(crypto) = self.crypto.as_ref() else {
            // 明文实例遇到加密文件（配置回滚场景）：段长可走、密文不可解
            // → 全部计 skipped（文件由 replay 保留）
            return (Vec::new(), count_segments(data) as u64);
        };
        let version = u16::from_le_bytes(data[8..10].try_into().expect("2 bytes"));
        let algo = u16::from_le_bytes(data[10..12].try_into().expect("2 bytes"));
        if version != JOURNAL_FORMAT_VERSION || algo != JOURNAL_ALGO_AES256_GCM {
            tracing::error!(
                version,
                algo,
                "{}",
                crate::i18n::tr("journal-unsupported-format")
            );
            // 版本/算法不支持：计段数使 skipped 显性可感知（文件保留）
            return (Vec::new(), count_segments(data) as u64);
        }
        let salt: [u8; 16] = data[12..JOURNAL_HEADER_LEN].try_into().expect("16 bytes");
        let (_, cipher) = crypto.cipher_for_salt(&salt);

        let mut records = Vec::new();
        let mut skipped = 0u64;
        let mut off = JOURNAL_HEADER_LEN;
        while off + SEGMENT_PREFIX_LEN <= data.len() {
            let ct_len = u32::from_le_bytes(
                data[off + NONCE_LEN..off + SEGMENT_PREFIX_LEN]
                    .try_into()
                    .expect("4 bytes"),
            ) as usize;
            let end = off + SEGMENT_PREFIX_LEN + ct_len;
            if end > data.len() {
                skipped += 1;
                break;
            }
            let Ok(nonce) = aes_gcm::Nonce::try_from(&data[off..off + NONCE_LEN]) else {
                skipped += 1;
                break;
            };
            match cipher.decrypt(&nonce, &data[off + SEGMENT_PREFIX_LEN..end]) {
                Ok(plaintext) => match serde_json::from_slice::<LogRecord>(&plaintext) {
                    Ok(record) => records.push(record),
                    Err(_) => skipped += 1,
                },
                Err(_) => skipped += 1,
            }
            off = end;
        }
        (records, skipped)
    }

    fn push_to_sender(&self, payload: &[u8]) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };
        match sender.push(payload) {
            Ok(()) => {
                self.pushed_count.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                let total = self.push_failures.fetch_add(1, Ordering::Relaxed) + 1;
                tracing::warn!(error = %e, total,
                    "{}", crate::i18n::tr("journal-push-failed-kept"));
            }
        }
    }

    fn current_size(&self) -> u64 {
        std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    /// 截断头部：从最旧记录起丢弃，直到腾出 `needed` 字节空间。
    ///
    /// 切点对齐记录边界：整行含行尾换行一并计入释放量，保留体必然以换行
    /// 结尾——下一条追加才不会与最后保留行融合成非法 JSON（此前 join("\n")
    /// 丢尾部换行，每次容量截断连带销毁两条记录）。按字节切分兼容含非
    /// UTF-8 残缺字节的文件（read_to_string 会整体失败、截断失效致超限增长）。
    fn truncate_head_for(&self, needed: u64) {
        let Ok(data) = std::fs::read(&self.path) else {
            return;
        };
        let mut cut = 0usize;
        let mut freed = 0u64;
        for line in data.split_inclusive(|b| *b == b'\n') {
            freed += line.len() as u64;
            cut += line.len();
            if freed >= needed {
                break;
            }
        }
        let tmp = self.path.with_extension("journal.tmp");
        if std::fs::write(&tmp, &data[cut..])
            .and_then(|_| std::fs::rename(&tmp, &self.path))
            .is_err()
        {
            // 截断失败：放弃保留，直接清空（宁可丢旧也不超限增长）
            let _ = std::fs::write(&self.path, b"");
        }
    }
}

fn header_conflict_args(path: &std::path::Path) -> crate::i18n::MsgArgs {
    let mut args = crate::i18n::MsgArgs::new();
    args.set("file", path.display().to_string());
    args
}

/// 加密 journal 写入句柄：unix 0600——加密产物与 key 文件同一约束
/// （默认 umask 的 0644 会把密文暴露给组/其他用户）。`append = true` 为
/// 段追加；否则 write + truncate（头创建 / tmp 重写 / 兜底重写）。
fn open_journal_file(path: &std::path::Path, append: bool) -> std::io::Result<std::fs::File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).write(true);
    if append {
        opts.append(true);
    } else {
        opts.truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

/// 数加密文件的完整段数（含尾部残段），供不可解密场景计数。
fn count_segments(data: &[u8]) -> usize {
    let mut count = 0usize;
    let mut off = JOURNAL_HEADER_LEN;
    while off + SEGMENT_PREFIX_LEN <= data.len() {
        let ct_len = u32::from_le_bytes(
            data[off + NONCE_LEN..off + SEGMENT_PREFIX_LEN]
                .try_into()
                .expect("4 bytes"),
        ) as usize;
        count += 1;
        if off + SEGMENT_PREFIX_LEN + ct_len > data.len() {
            break;
        }
        off += SEGMENT_PREFIX_LEN + ct_len;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike as _;

    fn record(msg: &str) -> LogRecord {
        let mut r = LogRecord::new(
            tracing::Level::ERROR,
            "journal::test".to_string(),
            msg.to_string(),
        );
        r.fields.insert("k".to_string(), serde_json::json!("v"));
        r.trace_id = Some("0123456789abcdef0123456789abcdef".to_string());
        r
    }

    #[test]
    fn test_spill_replay_roundtrip_preserves_fields() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = FallbackJournal::open(dir.path().join("fb.journal"));

        assert!(journal.spill(&record("first")));
        assert!(journal.spill(&record("second")));

        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 0);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].message, "first");
        assert_eq!(records[1].message, "second");
        assert_eq!(
            records[0].trace_id.as_deref(),
            Some("0123456789abcdef0123456789abcdef") // pragma: allowlist secret — 测试夹具 trace id
        );
        assert_eq!(records[0].fields.get("k"), Some(&serde_json::json!("v")));
        assert_eq!(records[0].timestamp.year(), chrono::Utc::now().year());

        // 重放后清空：二次重放为空
        let (records, _) = journal.replay();
        assert!(records.is_empty());
    }

    #[test]
    fn test_capacity_cap_drops_oldest() {
        let dir = tempfile::TempDir::new().unwrap();
        // 上限 1 KiB：每条约 300 字节，灌 8 条必然触发截断丢最旧
        let journal = FallbackJournal::with_limit(dir.path().join("fb.journal"), 1024);
        for i in 0..8 {
            assert!(journal.spill(&record(&format!("payload-{i}-{}", "x".repeat(40)))));
        }
        let (records, _) = journal.replay();
        assert!(
            records.len() < 8,
            "cap must drop oldest entries: {}",
            records.len()
        );
        // 保留的是最新的（payload-7 必在）
        assert!(
            records.iter().any(|r| r.message.starts_with("payload-7")),
            "newest entry must survive truncation"
        );
        assert!(
            records.iter().all(|r| !r.message.starts_with("payload-0")),
            "oldest entry must be dropped"
        );
    }

    /// 容量截断切点必须对齐记录边界（保留行尾换行）：旧实现 join("\n") 丢
    /// 尾部换行，截断后首条追加与最后保留行融合为一条非法 JSON——每次容量
    /// 截断连带销毁「最后保留行 + 新追加行」两条记录。
    #[test]
    fn test_truncate_cut_aligns_record_boundary() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("cut.journal");
        let journal = FallbackJournal::with_limit(&path, 1024);
        for i in 0..4 {
            assert!(journal.spill(&record(&format!("payload-{i}-{}", "x".repeat(40)))));
        }
        // 截断发生后立即追加：新记录必须与保留边界对齐，不得融合
        assert!(journal.spill(&record("after-truncate")));

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            raw.ends_with('\n'),
            "journal must stay newline-terminated after truncation + append"
        );
        for line in raw.lines() {
            assert!(
                serde_json::from_str::<LogRecord>(line).is_ok(),
                "every line must stay a valid JSON record after truncation, got: {line}"
            );
        }

        let (records, skipped) = journal.replay();
        assert_eq!(
            skipped, 0,
            "truncation must not produce fused/invalid lines"
        );
        let messages: Vec<_> = records.iter().map(|r| r.message.clone()).collect();
        assert!(
            messages.contains(&"after-truncate".to_string()),
            "record appended right after truncation must survive, got {messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.starts_with("payload-3")),
            "last kept record must stay intact (not fused), got {messages:?}"
        );
    }

    #[test]
    fn test_corrupt_lines_skipped_without_panic() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("fb.journal");
        journal_spill(&FallbackJournal::open(&path), &record("good-1"));
        // 半行损坏 + 合法 + 非JSON
        std::fs::write(
            &path,
            "{\"half\":\"line\n{{{{not-json\n\"line without braces\n",
        )
        .unwrap();
        let journal = FallbackJournal::open(&path);
        assert!(journal.spill(&record("good-2")));
        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 3, "corrupt lines counted and skipped");
        // good-1 已被上面的 fs::write 覆盖，仅 good-2 合法
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "good-2");
    }

    fn journal_spill(journal: &FallbackJournal, record: &LogRecord) {
        assert!(journal.spill(record));
    }

    #[test]
    fn test_replay_on_missing_file_is_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = FallbackJournal::open(dir.path().join("nonexistent.journal"));
        let (records, skipped) = journal.replay();
        assert!(records.is_empty());
        assert_eq!(skipped, 0);
    }

    // ========================================================================
    // journal 加密与推送（R-inklog-002）
    // nonce 唯一性策略（实现前固化）：随机 96-bit/段，CSPRNG 生成；
    // 单测以同 key 跨段/跨文件收集 nonce 断言无重复。
    // key 来源优先级：env > 配置文件；配置文件 key unix 权限必须 0600。
    // 显式启用加密而 key 缺失/无效：构造期显性失败（Err），禁止静默明文。
    // ========================================================================

    use base64::{Engine as _, engine::general_purpose};
    use serial_test::serial;

    /// 高熵 32 字节测试密钥（i*37+11 模 256 互异 → 熵 = log2(32) = 5 ≥ 4.0）。
    fn test_key_bytes(seed: u8) -> [u8; 32] {
        core::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(seed))
    }

    fn key_env_b64(var: &'static str, key: [u8; 32]) {
        unsafe {
            std::env::set_var(var, general_purpose::STANDARD.encode(key).as_str());
        }
    }

    /// 记录推送 mock：收集 payload 或按 fail 恒失败。
    struct RecordingSender {
        pushes: std::sync::Mutex<Vec<Vec<u8>>>,
        fail: bool,
    }

    impl RecordingSender {
        fn ok() -> Self {
            Self {
                pushes: std::sync::Mutex::new(Vec::new()),
                fail: false,
            }
        }
        fn failing() -> Self {
            Self {
                pushes: std::sync::Mutex::new(Vec::new()),
                fail: true,
            }
        }
    }

    impl JournalPushSender for RecordingSender {
        fn push(&self, payload: &[u8]) -> Result<(), InklogError> {
            if self.fail {
                return Err(InklogError::ChannelError("mock push failure".to_string()));
            }
            self.pushes.lock().unwrap().push(payload.to_vec());
            Ok(())
        }
    }

    /// 从 journal 原始字节解析出各段 nonce（跳过 28 字节头，走段定长）。
    fn parse_segment_nonces(data: &[u8]) -> Vec<[u8; 12]> {
        let mut nonces = Vec::new();
        let mut off = JOURNAL_HEADER_LEN;
        while off + 16 <= data.len() {
            let mut nonce = [0u8; 12];
            nonce.copy_from_slice(&data[off..off + 12]);
            let ct_len = u32::from_le_bytes(data[off + 12..off + 16].try_into().unwrap()) as usize;
            nonces.push(nonce);
            off += 16 + ct_len;
        }
        nonces
    }

    #[test]
    #[serial]
    fn test_encrypted_spill_replay_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("enc.journal");
        key_env_b64("INKLOG_TEST_JKEY_RT", test_key_bytes(11));

        let journal = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_RT".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(journal.spill(&record("enc-first")));
        assert!(journal.spill(&record("enc-second")));

        let raw = std::fs::read(&path).unwrap();
        assert!(
            raw.starts_with(&JOURNAL_MAGIC),
            "encrypted journal must start with magic header, got {raw:?}"
        );

        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 0, "encrypted roundtrip must lose nothing");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].message, "enc-first");
        assert_eq!(records[1].message, "enc-second");
        assert_eq!(
            records[0].trace_id.as_deref(),
            Some("0123456789abcdef0123456789abcdef") // pragma: allowlist secret — 测试夹具 trace id
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_RT");
        }
    }

    #[test]
    #[serial]
    fn test_legacy_plaintext_journal_replayed_by_crypto_instance() {
        // 旧明文 JSONL 文件必须可被启用加密的新版实例 replay（格式自适应）
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("legacy.journal");
        let r1 = record("legacy-1");
        let r2 = record("legacy-2");
        let body = format!(
            "{}\n{}\n",
            serde_json::to_string(&r1).unwrap(),
            serde_json::to_string(&r2).unwrap()
        );
        std::fs::write(&path, body).unwrap();

        key_env_b64("INKLOG_TEST_JKEY_LEGACY", test_key_bytes(12));
        let journal = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_LEGACY".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 0);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].message, "legacy-1");

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_LEGACY");
        }
    }

    #[test]
    #[serial]
    fn test_wrong_key_rejected_on_replay() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("wrongkey.journal");
        key_env_b64("INKLOG_TEST_JKEY_WK_A", test_key_bytes(21));
        key_env_b64("INKLOG_TEST_JKEY_WK_B", test_key_bytes(22));

        let writer = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_WK_A".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(writer.spill(&record("secret-one")));
        assert!(writer.spill(&record("secret-two")));

        let reader = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_WK_B".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        let (records, skipped) = reader.replay();
        assert!(records.is_empty(), "wrong key must yield no records");
        assert_eq!(skipped, 2, "undecryptable segments must be counted");
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !raw.contains("secret-one") && !raw.contains("secret-two"),
            "ciphertext must not contain plaintext"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_WK_A");
            std::env::remove_var("INKLOG_TEST_JKEY_WK_B");
        }
    }

    #[test]
    #[serial]
    fn test_plain_instance_on_encrypted_file_counts_skipped() {
        // 未配置加密的实例遇到加密文件：不可解密但可走段定长 → 全部计 skipped
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("mixed.journal");
        key_env_b64("INKLOG_TEST_JKEY_MIX", test_key_bytes(31));
        let writer = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_MIX".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(writer.spill(&record("m1")));
        assert!(writer.spill(&record("m2")));

        let plain = FallbackJournal::open(&path);
        let raw_before = std::fs::read(&path).unwrap();
        let (records, skipped) = plain.replay();
        assert!(records.is_empty());
        assert_eq!(skipped, 2, "segments must be counted, not silently dropped");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            raw_before,
            "config rollback (plain instance) must preserve the encrypted file"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_MIX");
        }
    }

    #[test]
    #[serial]
    fn test_missing_key_is_explicit_construction_failure() {
        // 显式启用加密而 key 缺失：构造期 Err（显性失败），禁止静默明文落盘
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("nokey.journal");
        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_MISSING");
        }
        let result = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_MISSING".to_string()),
                key_file: None,
            },
        );
        let Err(err) = result else {
            panic!("missing key must fail construction explicitly");
        };
        assert!(
            err.to_string().contains("INKLOG_TEST_JKEY_MISSING"),
            "error must name the missing key source, got: {err}"
        );
        assert!(
            !path.exists(),
            "no plaintext journal may be created when key is missing"
        );
    }

    #[test]
    #[serial]
    fn test_invalid_key_format_rejected_at_construction() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("badkey.journal");

        // Base64 可解码但长度不是 32 字节 → 无效
        unsafe {
            std::env::set_var(
                "INKLOG_TEST_JKEY_BAD_LEN",
                general_purpose::STANDARD.encode([0u8; 16]).as_str(),
            );
        }
        let result = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_BAD_LEN".to_string()),
                key_file: None,
            },
        );
        assert!(result.is_err(), "16-byte base64 key must be rejected");

        // 过短密码 → 无效
        unsafe {
            std::env::set_var("INKLOG_TEST_JKEY_SHORT_PWD", "short");
        }
        let result = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_SHORT_PWD".to_string()),
                key_file: None,
            },
        );
        assert!(result.is_err(), "short password must be rejected");

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_BAD_LEN");
            std::env::remove_var("INKLOG_TEST_JKEY_SHORT_PWD");
        }
    }

    #[test]
    #[serial]
    fn test_nonce_unique_across_segments_and_files_same_key() {
        // 固化的 nonce 唯一性策略（随机 96-bit/段）：同 key 跨段、跨文件不重复
        let dir = tempfile::TempDir::new().unwrap();
        key_env_b64("INKLOG_TEST_JKEY_NONCE", test_key_bytes(41));

        let j1 = FallbackJournal::with_crypto(
            dir.path().join("n1.journal"),
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_NONCE".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        for i in 0..64 {
            assert!(j1.spill(&record(&format!("nonce-a-{i}"))));
        }
        let j2 = FallbackJournal::with_crypto(
            dir.path().join("n2.journal"),
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_NONCE".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        for i in 0..64 {
            assert!(j2.spill(&record(&format!("nonce-b-{i}"))));
        }

        let mut nonces =
            parse_segment_nonces(&std::fs::read(dir.path().join("n1.journal")).unwrap());
        nonces.extend(parse_segment_nonces(
            &std::fs::read(dir.path().join("n2.journal")).unwrap(),
        ));
        assert_eq!(nonces.len(), 128, "every spill must emit one segment");
        let unique: std::collections::HashSet<[u8; 12]> = nonces.iter().copied().collect();
        assert_eq!(
            unique.len(),
            128,
            "same-key nonces must never repeat across segments and files"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_NONCE");
        }
    }

    #[test]
    #[serial]
    fn test_env_key_takes_priority_over_key_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("prio.journal");
        let key_file = dir.path().join("journal.key");
        std::fs::write(
            &key_file,
            general_purpose::STANDARD.encode(test_key_bytes(52)),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        // env 与 key file 同时存在：env 优先（用 env key 可解）
        key_env_b64("INKLOG_TEST_JKEY_PRIO", test_key_bytes(51));
        let writer = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_PRIO".to_string()),
                key_file: Some(key_file.clone()),
            },
        )
        .unwrap();
        assert!(writer.spill(&record("priority-check")));

        let env_reader = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_PRIO".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        let (records, skipped) = env_reader.replay();
        assert_eq!(skipped, 0, "env key must win over key file");
        assert_eq!(records.len(), 1);

        // 仅 key file（不注入 env）：解密失败 → skipped（证明写入用的是 env key）。
        // 注意：上方 env_reader.replay() 已清空文件，此处 spill 重建头后
        // 仅 1 段（密钥材料在 writer 实例内缓存，仍为 env key）。
        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_PRIO");
        }
        assert!(writer.spill(&record("after-env-removed")));
        let file_reader = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_PRIO".to_string()),
                key_file: Some(key_file.clone()),
            },
        )
        .unwrap();
        let (records, skipped) = file_reader.replay();
        assert!(records.is_empty());
        assert_eq!(skipped, 1, "file key must not decrypt env-key segments");
    }

    #[test]
    #[serial]
    fn test_key_file_requires_0600_permissions() {
        // unix：key file 权限必须 0600；组/其他可读 → 显性拒绝
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("perm.journal");
        let key_file = dir.path().join("journal.key");
        std::fs::write(
            &key_file,
            general_purpose::STANDARD.encode(test_key_bytes(61)),
        )
        .unwrap();

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_PERM");
        }
        let cfg = || JournalCryptoConfig {
            key_env: Some("INKLOG_TEST_JKEY_PERM".to_string()),
            key_file: Some(key_file.clone()),
        };

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o644)).unwrap();
            let result = FallbackJournal::with_crypto(&path, DEFAULT_JOURNAL_MAX_BYTES, cfg());
            let Err(err) = result else {
                panic!("group/other-readable key file must be rejected");
            };
            assert!(
                err.to_string().contains("0600") || err.to_string().contains("permission"),
                "permission error must be explicit, got: {err}"
            );

            std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let result = FallbackJournal::with_crypto(&path, DEFAULT_JOURNAL_MAX_BYTES, cfg());
        assert!(result.is_ok(), "0600 key file must be accepted");

        // file-only 配置可写可读（回归：key file 是合法唯一来源）
        result.unwrap().spill(&record("from-key-file"));
    }

    #[test]
    #[serial]
    fn test_push_sender_receives_spilled_records() {
        let dir = tempfile::TempDir::new().unwrap();
        let sender = RecordingSender::ok();
        let journal =
            FallbackJournal::open(dir.path().join("push.journal")).with_sender(Arc::new(sender));
        assert!(journal.spill(&record("push-1")));
        assert!(journal.spill(&record("push-2")));
        assert!(journal.spill(&record("push-3")));

        // 借用内部 mock 断言：通过 replay 验证落盘 + 通过 sender 计数验证推送
        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 0);
        assert_eq!(records.len(), 3, "spill must remain durable to disk");
        assert_eq!(journal.push_failure_count(), 0);
        assert_eq!(journal.pushed_count(), 3);
    }

    #[test]
    #[serial]
    fn test_push_sender_failure_counted_and_non_fatal() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = FallbackJournal::open(dir.path().join("pushfail.journal"))
            .with_sender(Arc::new(RecordingSender::failing()));
        assert!(
            journal.spill(&record("pf-1")),
            "push failure must not fail spill"
        );
        assert!(journal.spill(&record("pf-2")));
        assert!(journal.spill(&record("pf-3")));

        let (records, _) = journal.replay();
        assert_eq!(records.len(), 3, "records must remain on disk");
        assert_eq!(journal.push_failure_count(), 3, "failures must be visible");
    }

    #[test]
    #[serial]
    fn test_capacity_truncate_preserves_encrypted_header_and_latest() {
        // 加密 journal 超限截断：保留头（新 salt 生效）且最新记录可解
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("trunc.journal");
        key_env_b64("INKLOG_TEST_JKEY_TRUNC", test_key_bytes(71));
        let journal = FallbackJournal::with_crypto(
            &path,
            1024,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_TRUNC".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        for i in 0..8 {
            assert!(journal.spill(&record(&format!("trunc-{i}-{}", "x".repeat(40)))));
        }
        assert!(journal.spill(&record("trunc-newest")));
        let raw = std::fs::read(&path).unwrap();
        assert!(
            raw.starts_with(&JOURNAL_MAGIC),
            "header must survive truncation"
        );

        let (records, _) = journal.replay();
        assert!(records.len() < 9, "capacity must drop oldest");
        assert!(
            records
                .iter()
                .any(|r| r.message.starts_with("trunc-newest")),
            "newest record must survive and decrypt"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_TRUNC");
        }
    }

    #[test]
    #[serial]
    fn test_wrong_key_replay_preserves_file() {
        // 错误密钥全军覆没：文件必须原样保留（此前的无条件清空会销毁记录，
        // 正确密钥恢复后无法挽回），skipped 显性可感知
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("wrongkey_keep.journal");
        key_env_b64("INKLOG_TEST_JKEY_KEEP_A", test_key_bytes(21));
        key_env_b64("INKLOG_TEST_JKEY_KEEP_B", test_key_bytes(22));

        let writer = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_KEEP_A".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(writer.spill(&record("secret-one")));
        assert!(writer.spill(&record("secret-two")));
        let raw_before = std::fs::read(&path).unwrap();

        let reader = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_KEEP_B".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        let (records, skipped) = reader.replay();
        assert!(records.is_empty(), "wrong key must yield no records");
        assert_eq!(skipped, 2, "undecryptable segments must be counted");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            raw_before,
            "undecryptable journal must be preserved byte-for-byte"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_KEEP_A");
            std::env::remove_var("INKLOG_TEST_JKEY_KEEP_B");
        }
    }

    /// 部分段不可解（中途换钥）时不得清空文件：可重放记录非空 ≠ 全部成功，
    /// 仅 `skipped == 0` 才清空——不可解段可能随密钥恢复/排查事后可解，
    /// 字节销毁不可逆（旧实现「有任一记录成功就清空」会销毁不可解段）。
    #[test]
    #[serial]
    fn test_partial_decrypt_failure_preserves_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("partial.journal");
        key_env_b64("INKLOG_TEST_JKEY_PART_A", test_key_bytes(23));
        let writer_a = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_PART_A".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(writer_a.spill(&record("rotated-out-1")));
        assert!(writer_a.spill(&record("rotated-out-2")));

        // 中途换钥：同路径新实例复用文件头盐，但用新钥派生续写
        key_env_b64("INKLOG_TEST_JKEY_PART_B", test_key_bytes(24));
        let writer_b = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_PART_B".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(writer_b.spill(&record("current-1")));
        assert!(writer_b.spill(&record("current-2")));

        let raw_before = std::fs::read(&path).unwrap();
        let (records, skipped) = writer_b.replay();
        assert_eq!(skipped, 2, "old-key segments must be counted as skipped");
        assert_eq!(records.len(), 2, "current-key segments must replay");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            raw_before,
            "partial decrypt failure must preserve the file byte-for-byte"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_PART_A");
            std::env::remove_var("INKLOG_TEST_JKEY_PART_B");
        }
    }

    #[test]
    #[serial]
    fn test_unsupported_version_preserves_file_and_counts_skipped() {
        // 版本/算法不支持的加密文件：保留 + 段计数显性化（此前清空且 skipped=0）
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("futurever.journal");
        key_env_b64("INKLOG_TEST_JKEY_FVER", test_key_bytes(81));
        let journal = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_FVER".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(journal.spill(&record("v1-record")));
        let raw_before = std::fs::read(&path).unwrap();

        // 手工把版本号改成未来版本（其余结构不动）
        let mut future = raw_before.clone();
        future[8..10].copy_from_slice(&999u16.to_le_bytes());
        std::fs::write(&path, &future).unwrap();

        let (records, skipped) = journal.replay();
        assert!(records.is_empty());
        assert_eq!(
            skipped, 1,
            "segments must be counted on unsupported version"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            future,
            "file must be preserved"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_FVER");
        }
    }

    #[test]
    #[serial]
    fn test_plain_spill_refuses_encrypted_file() {
        // 配置回滚护栏：明文实例不得向加密 journal 追加明文（破坏段定长
        // 结构 + 明文混入加密文件）；spill 报失败，文件字节不变
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("rollback.journal");
        key_env_b64("INKLOG_TEST_JKEY_ROLL", test_key_bytes(91));
        let writer = FallbackJournal::with_crypto(
            &path,
            DEFAULT_JOURNAL_MAX_BYTES,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_ROLL".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        assert!(writer.spill(&record("encrypted-1")));
        let raw_before = std::fs::read(&path).unwrap();

        let plain = FallbackJournal::open(&path);
        assert!(
            !plain.spill(&record("plaintext-after-rollback")),
            "plaintext spill onto encrypted journal must fail explicitly"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            raw_before,
            "encrypted journal must not be modified by plaintext spill"
        );
        // 明文实例 replay：保留文件（文件头未被改写为空明文）
        let (records, skipped) = plain.replay();
        assert!(records.is_empty());
        assert!(skipped >= 1);
        assert!(
            std::fs::read(&path).unwrap().starts_with(&JOURNAL_MAGIC),
            "config rollback must not destroy the encrypted journal"
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_ROLL");
        }
    }

    #[test]
    #[serial]
    #[cfg(unix)]
    fn test_encrypted_journal_files_created_0600() {
        // 加密 journal 的头创建、段追加、截断 tmp+rename 产物全部 0600
        //（默认 umask 0644 会把密文暴露给组/其他用户）
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("perm0600.journal");
        key_env_b64("INKLOG_TEST_JKEY_0600", test_key_bytes(82));
        let journal = FallbackJournal::with_crypto(
            &path,
            1024,
            JournalCryptoConfig {
                key_env: Some("INKLOG_TEST_JKEY_0600".to_string()),
                key_file: None,
            },
        )
        .unwrap();
        for i in 0..8 {
            assert!(journal.spill(&record(&format!("perm-{i}-{}", "x".repeat(40)))));
        }
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "encrypted journal must be 0600, got {:o}",
            mode & 0o777
        );

        // 触发超限截断（tmp + rename 重建文件）：权限不得回退
        for i in 0..8 {
            assert!(journal.spill(&record(&format!("perm-more-{i}-{}", "y".repeat(60)))));
        }
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "truncated (tmp+rename) journal must stay 0600, got {:o}",
            mode & 0o777
        );

        unsafe {
            std::env::remove_var("INKLOG_TEST_JKEY_0600");
        }
    }

    // ------------------------------------------------------------------
    // TcpJournalPusher：断线缓冲 + 自动重连 + 满则丢最旧（net-sink 范式）
    // ------------------------------------------------------------------

    /// 后台 TCP 服务端：收集收到的行；首个连接可按阈值断开模拟对端重启。
    struct PushTestServer {
        addr: std::net::SocketAddr,
        received: Arc<std::sync::Mutex<Vec<String>>>,
        shutdown: Arc<std::sync::atomic::AtomicBool>,
        handle: std::thread::JoinHandle<()>,
    }

    impl PushTestServer {
        fn spawn(close_after_lines: Option<usize>) -> Self {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let received = Arc::new(std::sync::Mutex::new(Vec::new()));
            let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let (received_clone, shutdown_clone) = (received.clone(), shutdown.clone());
            let handle = std::thread::spawn(move || {
                let mut accepted = 0usize;
                for stream in listener.incoming() {
                    if shutdown_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    let Ok(mut stream) = stream else { continue };
                    accepted += 1;
                    let kill_after = if accepted == 1 {
                        close_after_lines
                    } else {
                        None
                    };
                    let mut conn_lines = 0usize;
                    loop {
                        let mut buf = [0u8; 4096];
                        match std::io::Read::read(&mut stream, &mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                let text = String::from_utf8_lossy(&buf[..n]);
                                for line in text.lines() {
                                    received_clone.lock().unwrap().push(line.to_string());
                                    conn_lines += 1;
                                }
                                if let Some(limit) = kill_after
                                    && conn_lines >= limit
                                {
                                    let _ = stream.shutdown(std::net::Shutdown::Both);
                                    break;
                                }
                            }
                        }
                    }
                    if shutdown_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                }
            });
            Self {
                addr,
                received,
                shutdown,
                handle,
            }
        }

        fn lines(&self) -> Vec<String> {
            self.received.lock().unwrap().clone()
        }

        /// 置停标志、打断 accept 并回收服务端线程。
        fn stop(self) {
            self.shutdown
                .store(true, std::sync::atomic::Ordering::Relaxed);
            let _ = std::net::TcpStream::connect(self.addr);
            let _ = self.handle.join();
        }
    }

    fn pusher(addr: String, capacity: usize) -> TcpJournalPusher {
        TcpJournalPusher::new(TcpJournalPusherConfig {
            addr,
            connect_timeout: std::time::Duration::from_secs(1),
            // 收紧 flush 预算（connect + write 超时）：对端不可达的负路径测试
            // 不必烧满生产默认的 3s+3s
            write_timeout: std::time::Duration::from_millis(100),
            buffer_capacity: capacity,
            ..Default::default()
        })
        .unwrap()
    }

    /// 轮询等待断线缓冲达到 `min_len`：`buffered_len` 是瞬时观测，投递线程
    /// 换出批量在 `state` 锁外做建连/退避重判期间缓冲短暂为空，全量测试
    /// 高负载下投递线程被抢占会拉长该窗口，单次读可能撞上；截止时间内
    /// 重试吸收瞬时读数，超时仍不满足才判失败。容量上界使 `>=` 等价于
    /// 稳态条数相等。
    fn wait_for_buffered_len(pusher: &TcpJournalPusher, min_len: usize, msg: &str) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let len = pusher.buffered_len();
            if len >= min_len {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "{msg}: buffered_len={len} (expected >= {min_len}), dropped_total={}",
                pusher.dropped_total()
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn test_tcp_pusher_rejects_invalid_addr() {
        let result = TcpJournalPusher::new(TcpJournalPusherConfig {
            addr: "no-port-here".to_string(),
            ..Default::default()
        });
        assert!(result.is_err(), "addr without port must be rejected");
    }

    #[test]
    fn test_tcp_pusher_addr_validation_is_syntax_only() {
        // 构造期仅做 host:port 语法校验：不可解析的 DNS 名不得让「尽力而为」
        // 推送端口以 ConfigError 终止整个构建（解析延迟到投递线程 connect）
        for good in ["downstream.invalid:5170", "[::1]:5170", "10.0.0.1:1"] {
            assert!(
                TcpJournalPusher::new(TcpJournalPusherConfig {
                    addr: good.to_string(),
                    ..Default::default()
                })
                .is_ok(),
                "syntactically valid addr must be accepted: {good}"
            );
        }
        for bad in [":5170", "host:notaport", "no-port-here", ""] {
            assert!(
                TcpJournalPusher::new(TcpJournalPusherConfig {
                    addr: bad.to_string(),
                    ..Default::default()
                })
                .is_err(),
                "invalid addr must be rejected: {bad}"
            );
        }
    }

    /// 对端接受连接但从不排空：写超时必须把连接判死并走缓冲路径，
    /// push 不得无限阻塞（此前同步 write_all 会冻结关键日志线程）。
    #[test]
    fn test_tcp_pusher_write_timeout_on_non_draining_peer() {
        use std::sync::atomic::AtomicUsize;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        // 每个连接：立即计数后持有不读（不排空）；线程随测试进程结束回收
        let accepted = Arc::new(AtomicUsize::new(0));
        let accepted_counter = Arc::clone(&accepted);
        let acceptor = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                accepted_counter.fetch_add(1, Ordering::Relaxed);
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    drop(stream);
                });
            }
        });

        let pusher = TcpJournalPusher::new(TcpJournalPusherConfig {
            addr: addr.to_string(),
            connect_timeout: std::time::Duration::from_secs(1),
            write_timeout: std::time::Duration::from_millis(100),
            buffer_capacity: 4096,
            transport_security: TransportSecurity::None,
        })
        .unwrap();

        let start = std::time::Instant::now();
        // 128KiB × 256 = 32MiB：远超单连接内核缓冲（发送/接收合计 ≤10MiB 量级），
        // 不排空对端必然触发写超时 → 连接弃用 → 重连补发
        let chunk = vec![b'x'; 128 * 1024];
        for _ in 0..256 {
            pusher.push(&chunk).unwrap();
        }
        // push 只入队（O(1)），整体必须远快于任何网络停滞
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "push must never block on a non-draining peer, took {:?}",
            start.elapsed()
        );

        // 写超时弃用连接后投递线程必然重连：对端不关闭任何连接，
        // 服务端看到 ≥2 次连接建立 ⟺ 超时弃连 + 重连路径被触发
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while accepted.load(Ordering::Relaxed) < 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(
            accepted.load(Ordering::Relaxed) >= 2,
            "write timeout must discard the stuck connection and reconnect (accepted={})",
            accepted.load(Ordering::Relaxed)
        );
        assert_eq!(
            pusher.dropped_total(),
            0,
            "backlog under capacity must not drop"
        );
        drop(pusher);
        // acceptor 线程阻塞在 accept 上，随测试进程回收（不 join）
        drop(acceptor);
    }

    #[test]
    fn test_tcp_pusher_reconnect_replays_in_order() {
        let server = PushTestServer::spawn(Some(1));
        let pusher = pusher(server.addr.to_string(), 100);

        const LINES: [&str; 4] = ["l0", "l1-buffered", "l2-buffered", "l3-after"];
        for (i, line) in LINES.iter().enumerate() {
            if i == 1 {
                // 让服务端断开先于下一笔写入可见（TCP 写后知错窗口）
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            pusher.push(line.as_bytes()).unwrap();
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while server.lines().len() < LINES.len() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let lines = server.lines();
        assert_eq!(
            lines.len(),
            LINES.len(),
            "all pushed lines must arrive (buffered replayed), got {lines:?}"
        );
        for (i, line) in LINES.iter().enumerate() {
            assert!(
                lines[i].contains(line),
                "FIFO replay must keep order, expected {line} at {i}, got {lines:?}"
            );
        }
        pusher.flush_pending();
        // 先 drop 推送器关闭连接：服务端 read 返回 EOF 后线程才可退出，
        // stop 的 join 才不会死等
        drop(pusher);
        server.stop();
    }

    #[test]
    fn test_tcp_pusher_unreachable_buffers_without_error() {
        // 无网络环境：push 不报错、记录进缓冲、可观测
        let pusher = pusher("127.0.0.1:1".to_string(), 100);
        for i in 0..3 {
            pusher.push(format!("offline-{i}").as_bytes()).unwrap();
        }
        wait_for_buffered_len(&pusher, 1, "offline pushes must be buffered, not dropped");
        assert_eq!(pusher.dropped_total(), 0, "nothing dropped under capacity");
        pusher.flush_pending();
    }

    #[test]
    fn test_tcp_pusher_full_buffer_drops_oldest() {
        let pusher = pusher("127.0.0.1:1".to_string(), 2);
        for i in 0..5 {
            pusher.push(format!("overflow-{i}").as_bytes()).unwrap();
        }
        wait_for_buffered_len(&pusher, 2, "bounded capacity must hold");
        assert_eq!(pusher.dropped_total(), 3, "overflow must drop oldest");
        pusher.flush_pending();
    }

    /// 对端不可达（connect 拒绝）：退避期投递线程不得持锁忙自旋——连续
    /// push 每笔 <100ms、Drop（shutdown）有限时间返回（安全复审探针 ①）。
    #[test]
    fn test_tcp_pusher_unreachable_peer_push_not_blocked_by_backoff() {
        // 绑定后立即丢弃：得到一个必然连接拒绝的新鲜端口
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let pusher = Arc::new(
            TcpJournalPusher::new(TcpJournalPusherConfig {
                addr: addr.to_string(),
                connect_timeout: std::time::Duration::from_millis(200),
                write_timeout: std::time::Duration::from_millis(100),
                buffer_capacity: 4096,
                transport_security: TransportSecurity::None,
            })
            .unwrap(),
        );

        // 预热：让投递线程完成首次 connect 失败并进入退避
        pusher.push(b"warmup".as_slice()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));

        // 独立线程计时：若投递线程持锁自旋，push 饿死（通道超时显形，不挂死测试）
        let (tx, rx) = std::sync::mpsc::channel();
        let timed = Arc::clone(&pusher);
        let timer = std::thread::spawn(move || {
            for i in 0..10 {
                let start = std::time::Instant::now();
                timed.push(format!("spin-{i}").as_bytes()).unwrap();
                tx.send(start.elapsed()).unwrap();
            }
        });
        for _ in 0..10 {
            let elapsed = rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .expect("push must not block while delivery thread is in backoff");
            assert!(
                elapsed < std::time::Duration::from_millis(100),
                "each push must stay O(1) during peer-unreachable backoff, took {elapsed:?}"
            );
        }
        timer.join().unwrap();
        wait_for_buffered_len(&pusher, 10, "all pushes must be buffered");
        assert_eq!(pusher.dropped_total(), 0);

        // shutdown（Drop）在对端不可达 + 退避期必须有限时间返回
        let (dropped_tx, dropped_rx) = std::sync::mpsc::channel();
        let dropped = Arc::clone(&pusher);
        let dropper = std::thread::spawn(move || {
            drop(dropped);
            dropped_tx.send(()).unwrap();
        });
        dropped_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("drop must return in bounded time while peer unreachable");
        dropper.join().unwrap();
    }

    /// 对端接受连接但不排空（安全复审探针 ②）：积压远超内核缓冲、投递线程
    /// 处于 写满→写超时→弃连→重连 循环时，连续 push 每笔 <100ms——网络 IO
    /// 必须在 state 锁外执行（旧实现投递全程持锁，push 阻塞至积压排空）。
    #[test]
    fn test_tcp_pusher_backlog_over_kernel_buffer_push_latency_bounded() {
        use std::sync::atomic::AtomicUsize;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        let acceptor = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                counter.fetch_add(1, Ordering::Relaxed);
                std::thread::spawn(move || {
                    // 持有不读：接收窗口关闭，发送端写必然阻塞至写超时
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    drop(stream);
                });
            }
        });

        let pusher = Arc::new(
            TcpJournalPusher::new(TcpJournalPusherConfig {
                addr: addr.to_string(),
                connect_timeout: std::time::Duration::from_millis(500),
                write_timeout: std::time::Duration::from_millis(200),
                buffer_capacity: 4096,
                transport_security: TransportSecurity::None,
            })
            .unwrap(),
        );

        // 独立线程灌积压 8 MiB（≫ 回环内核收发缓冲）→ 停一拍让投递线程
        // 写满内核缓冲进入超时-重连循环 → 再连续 push 计时
        let (tx, rx) = std::sync::mpsc::channel();
        let timed = Arc::clone(&pusher);
        let timer = std::thread::spawn(move || {
            let chunk = vec![b'x'; 64 * 1024];
            for i in 0..138 {
                let start = std::time::Instant::now();
                if i < 128 {
                    timed.push(&chunk).unwrap();
                } else {
                    timed.push(format!("latency-{i}").as_bytes()).unwrap();
                }
                tx.send((i, start.elapsed())).unwrap();
                if i == 127 {
                    std::thread::sleep(std::time::Duration::from_millis(700));
                }
            }
        });
        for _ in 0..138 {
            let (i, elapsed) = rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .expect("push must not block while delivery thread drains a stuck peer");
            if i >= 128 {
                assert!(
                    elapsed < std::time::Duration::from_millis(100),
                    "push latency must stay O(1) with backlog over kernel buffer, took {elapsed:?}"
                );
            }
        }
        timer.join().unwrap();
        assert_eq!(
            pusher.dropped_total(),
            0,
            "backlog under capacity must not drop"
        );
        drop(pusher);
        drop(acceptor);
    }
}
