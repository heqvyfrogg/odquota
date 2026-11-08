//! Cross-platform encrypted credential store.
//!
//! The store is a single binary blob holding the whole account document, written
//! atomically (temp file + rename, mode 0600).
//!
//! Encryption differs by platform because the available OS facility differs:
//!
//! * Windows: DPAPI (`CryptProtectData`). Bound to the current user and machine,
//!   so it needs no password and cannot be copied to another machine.
//! * Linux/macOS: AES-256-GCM with a key derived by PBKDF2-HMAC-SHA256 from
//!   `ODQUOTA_PASSPHRASE`. The passphrase is read from the environment or an
//!   interactive prompt, never from argv.
//! * If no passphrase is available on Unix, the store holds plain JSON with mode
//!   0600 and reports `encrypted: false`. It never claims to be encrypted when
//!   it is not.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const STORE_VERSION: u32 = 1;

/// Envelope magic for the passphrase-encrypted format.
#[cfg(not(windows))]
const MAGIC: &[u8; 4] = b"ODQ1";
#[cfg(not(windows))]
const SALT_LEN: usize = 16;
#[cfg(not(windows))]
const NONCE_LEN: usize = 12;
#[cfg(not(windows))]
const PBKDF2_ITERATIONS: u32 = 200_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    pub label: String,
    pub control_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_key: Option<String>,
    pub api_url: String,
    pub link_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    pub api_url_source: String,
    pub added_at: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDocument {
    pub version: u32,
    pub accounts: Vec<AccountEntry>,
}

impl Default for StoreDocument {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            accounts: Vec::new(),
        }
    }
}

impl StoreDocument {
    pub fn find(&self, label: &str) -> Option<&AccountEntry> {
        self.accounts.iter().find(|a| a.label == label)
    }
}

/// How the bytes on disk are protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    /// Windows DPAPI, no passphrase required.
    Dpapi,
    /// Passphrase-derived AES-256-GCM.
    Passphrase,
    /// Plain JSON, 0600 only.
    Plain,
}

impl Protection {
    pub fn is_encrypted(self) -> bool {
        !matches!(self, Protection::Plain)
    }

    pub fn name(self) -> &'static str {
        match self {
            Protection::Dpapi => "dpapi",
            Protection::Passphrase => "passphrase",
            Protection::Plain => "plain",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    path: PathBuf,
    protection: Protection,
    #[cfg(not(windows))]
    passphrase: Option<String>,
}

impl Store {
    /// Resolve the store location and the protection mode that will be used.
    ///
    /// `ODQUOTA_HOME` overrides the whole directory so tests and multi-account
    /// probes never touch the real profile data.
    pub fn resolve() -> Result<Self, AppError> {
        let dir = match std::env::var_os("ODQUOTA_HOME") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => default_data_dir()?,
        };
        Ok(Self::with_path(dir.join("accounts.dat")))
    }

    /// Build a store for an explicit path (used by tests).
    pub fn with_path(path: PathBuf) -> Self {
        let passphrase = read_passphrase();
        let protection = if cfg!(windows) {
            Protection::Dpapi
        } else if passphrase.is_some() {
            Protection::Passphrase
        } else {
            Protection::Plain
        };
        Self {
            path,
            protection,
            #[cfg(not(windows))]
            passphrase,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn protection(&self) -> Protection {
        self.protection
    }

    pub fn encrypted(&self) -> bool {
        self.protection.is_encrypted()
    }

    /// Load the document. A missing or empty file yields an empty document,
    /// which is a legitimate first-run state rather than an error.
    pub fn load(&self) -> Result<StoreDocument, AppError> {
        let raw = match fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(StoreDocument::default())
            }
            Err(e) => {
                return Err(AppError::store(format!(
                    "保存ファイルを読めません {}: {e}",
                    self.path.display()
                )))
            }
        };
        if raw.is_empty() {
            return Ok(StoreDocument::default());
        }

        let plain = self.decrypt(&raw)?;
        let doc: StoreDocument = serde_json::from_slice(&plain).map_err(|e| {
            AppError::store(format!(
                "保存ファイルの JSON が壊れています {}: {e}",
                self.path.display()
            ))
        })?;
        if doc.version != STORE_VERSION {
            return Err(AppError::store(format!(
                "保存ファイルの version {} は未対応です（対応: {STORE_VERSION}）",
                doc.version
            )));
        }
        Ok(doc)
    }

    /// Write the document atomically: a sibling temp file is written, has its
    /// permissions tightened, then renamed over the target.
    pub fn save(&self, doc: &StoreDocument) -> Result<(), AppError> {
        let plain = serde_json::to_vec(doc)
            .map_err(|e| AppError::store(format!("保存内容を JSON 化できません: {e}")))?;
        let bytes = self.encrypt(&plain)?;

        let parent = self
            .path
            .parent()
            .ok_or_else(|| AppError::store("保存先ディレクトリを解決できません"))?;
        fs::create_dir_all(parent).map_err(|e| {
            AppError::store(format!("保存先を作れません {}: {e}", parent.display()))
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }

        let tmp = parent.join(format!(".accounts.{}.tmp", std::process::id()));
        {
            let mut f = fs::File::create(&tmp)
                .map_err(|e| AppError::store(format!("一時ファイルを作れません: {e}")))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
            }
            f.write_all(&bytes)
                .map_err(|e| AppError::store(format!("一時ファイルへ書けません: {e}")))?;
            f.flush()
                .map_err(|e| AppError::store(format!("一時ファイルを同期できません: {e}")))?;
        }

        fs::rename(&tmp, &self.path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            AppError::store(format!(
                "保存ファイルを置換できません {}: {e}",
                self.path.display()
            ))
        })?;
        Ok(())
    }

    fn decrypt(&self, raw: &[u8]) -> Result<Vec<u8>, AppError> {
        // The passphrase envelope is self-describing, so a file can be read on
        // any platform that can supply the passphrase.
        #[cfg(not(windows))]
        if raw.len() >= 4 && &raw[..4] == MAGIC {
            return self.decrypt_passphrase(raw);
        }
        #[cfg(windows)]
        {
            dpapi::unprotect(raw)
                .map_err(|e| AppError::store(format!("DPAPI 復号に失敗しました: {e}")))
        }
        #[cfg(not(windows))]
        {
            // Not one of our envelopes: treat it as the plain JSON store.
            Ok(raw.to_vec())
        }
    }

    fn encrypt(&self, plain: &[u8]) -> Result<Vec<u8>, AppError> {
        #[cfg(windows)]
        {
            match self.protection {
                Protection::Dpapi => dpapi::protect(plain)
                    .map_err(|e| AppError::store(format!("DPAPI 暗号化に失敗しました: {e}"))),
                _ => Ok(plain.to_vec()),
            }
        }
        #[cfg(not(windows))]
        {
            match self.protection {
                Protection::Passphrase => self.encrypt_passphrase(plain),
                _ => Ok(plain.to_vec()),
            }
        }
    }

    #[cfg(not(windows))]
    fn encrypt_passphrase(&self, plain: &[u8]) -> Result<Vec<u8>, AppError> {
        use crate::crypto::{aes_gcm_seal, pbkdf2_key, random_bytes};

        let pass = self
            .passphrase
            .as_deref()
            .ok_or_else(|| AppError::store("パスフレーズが未設定です（ODQUOTA_PASSPHRASE）"))?;
        let salt = random_bytes(SALT_LEN)?;
        let nonce = random_bytes(NONCE_LEN)?;
        let key = pbkdf2_key(pass, &salt, PBKDF2_ITERATIONS)?;
        let sealed = aes_gcm_seal(&key, &nonce, plain)?;

        let mut out = Vec::with_capacity(4 + SALT_LEN + NONCE_LEN + sealed.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&sealed);
        Ok(out)
    }

    #[cfg(not(windows))]
    fn decrypt_passphrase(&self, raw: &[u8]) -> Result<Vec<u8>, AppError> {
        use crate::crypto::{aes_gcm_open, pbkdf2_key};

        let pass = self.passphrase.as_deref().ok_or_else(|| {
            AppError::store(
                "保存ファイルはパスフレーズで暗号化されています。ODQUOTA_PASSPHRASE を設定してください",
            )
        })?;
        let body = &raw[4..];
        if body.len() < SALT_LEN + NONCE_LEN {
            return Err(AppError::store("保存ファイルのヘッダが短すぎます"));
        }
        let salt = &body[..SALT_LEN];
        let nonce = &body[SALT_LEN..SALT_LEN + NONCE_LEN];
        let sealed = &body[SALT_LEN + NONCE_LEN..];
        let key = pbkdf2_key(pass, salt, PBKDF2_ITERATIONS)?;
        aes_gcm_open(&key, nonce, sealed).map_err(|_| {
            AppError::store("復号に失敗しました（パスフレーズが違うか、ファイルが壊れています）")
        })
    }
}

/// `ODQUOTA_PASSPHRASE` if set and non-empty. Never read from argv.
fn read_passphrase() -> Option<String> {
    if cfg!(windows) {
        // DPAPI makes a passphrase unnecessary on Windows.
        return None;
    }
    std::env::var("ODQUOTA_PASSPHRASE")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn default_data_dir() -> Result<PathBuf, AppError> {
    if cfg!(windows) {
        let appdata = std::env::var_os("APPDATA")
            .ok_or_else(|| AppError::store("APPDATA を解決できません（保存先を決められません）"))?;
        return Ok(PathBuf::from(appdata).join("odquota"));
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("odquota"));
        }
    }
    let home = std::env::var_os("HOME")
        .ok_or_else(|| AppError::store("HOME を解決できません（保存先を決められません）"))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("odquota"))
}

#[cfg(windows)]
mod dpapi {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn blob(bytes: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as *mut core::ffi::c_void);
        v
    }

    pub fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptProtectData(
                &blob(plain),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        if ok == 0 {
            return Err(format!(
                "CryptProtectData: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(unsafe { take(out) })
    }

    pub fn unprotect(cipher: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptUnprotectData(
                &blob(cipher),
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        if ok == 0 {
            return Err(format!(
                "CryptUnprotectData: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(unsafe { take(out) })
    }
}
