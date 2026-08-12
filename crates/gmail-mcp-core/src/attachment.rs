//! Attachment policy: direct-delivery threshold, temporary file output,
//! filename sanitization, and retention cleanup.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::Error;

/// Default direct-delivery threshold (25 MB).
pub const DEFAULT_DIRECT_THRESHOLD: u64 = 25 * 1024 * 1024;
/// Default temporary attachment retention (24 hours).
pub const DEFAULT_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);

/// Attachment storage policy.
#[derive(Debug, Clone)]
pub struct AttachmentPolicy {
    /// Directory for temporary attachment files.
    pub cache_dir: PathBuf,
    /// Attachments at or below this size are returned directly; larger ones
    /// are written to `cache_dir`.
    pub direct_threshold: u64,
    /// How long temporary attachment files are kept before cleanup.
    pub retention: Duration,
}

impl Default for AttachmentPolicy {
    fn default() -> Self {
        AttachmentPolicy {
            cache_dir: default_cache_dir(),
            direct_threshold: DEFAULT_DIRECT_THRESHOLD,
            retention: DEFAULT_RETENTION,
        }
    }
}

/// The default temporary attachment directory: `~/.cache/gmail-mcp/attachments`.
pub fn default_cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".cache"))
                .unwrap_or_else(|| PathBuf::from(".cache"))
        });
    base.join("gmail-mcp").join("attachments")
}

impl AttachmentPolicy {
    /// Sanitize an untrusted attachment filename.
    ///
    /// Removes path separators, `..`, control characters, and other unsafe
    /// path characters while preserving a useful name.
    pub fn sanitize_filename(&self, name: &str) -> String {
        let mut out: String = name
            .chars()
            .map(|c| {
                if c.is_control()
                    || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
                {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        // Collapse any path-traversal residue and trim unsafe edge characters.
        while out.contains("..") {
            out = out.replace("..", "_");
        }
        let out = out.trim_matches(['.', ' ', '\t', '_']).to_string();
        if out.is_empty() {
            "attachment".to_string()
        } else {
            out
        }
    }

    /// Whether content of the given size should be returned directly.
    pub fn should_return_inline(&self, size: u64) -> bool {
        size <= self.direct_threshold
    }

    /// Write attachment content to a temporary file under the cache directory.
    ///
    /// The final path is guaranteed to remain under `cache_dir`.
    pub fn write_temp(&self, filename: &str, data: &[u8]) -> Result<PathBuf, Error> {
        let safe = self.sanitize_filename(filename);
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
            Error::Internal(format!(
                "failed to create attachment cache dir {}: {e}",
                self.cache_dir.display()
            ))
        })?;
        // Unique suffix avoids collisions between same-named attachments.
        let unique = format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            safe
        );
        let path = self.cache_dir.join(unique);
        std::fs::write(&path, data).map_err(|e| {
            Error::Internal(format!(
                "failed to write attachment {}: {e}",
                path.display()
            ))
        })?;
        Ok(path)
    }

    /// Remove temporary attachment files older than the retention period.
    ///
    /// Returns the number of files removed.
    pub fn cleanup_expired(&self) -> Result<usize, Error> {
        let now = std::time::SystemTime::now();
        let mut removed = 0;
        if !self.cache_dir.exists() {
            return Ok(0);
        }
        for entry in std::fs::read_dir(&self.cache_dir)
            .map_err(|e| Error::Internal(format!("failed to read attachment cache: {e}")))?
        {
            let entry = entry.map_err(|e| Error::Internal(format!("cache read error: {e}")))?;
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !meta.is_file() {
                continue;
            }
            let modified = match meta.modified() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if now
                .duration_since(modified)
                .map(|d| d > self.retention)
                .unwrap_or(false)
                && std::fs::remove_file(entry.path()).is_ok()
            {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Ensure a resolved path stays under the cache directory.
    pub fn is_within_cache(&self, path: &Path) -> bool {
        path.starts_with(&self.cache_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_unsafe_names() {
        let p = AttachmentPolicy::default();
        assert_eq!(p.sanitize_filename("../../etc/passwd"), "etc_passwd");
        assert_eq!(p.sanitize_filename("a/b\\c"), "a_b_c");
        assert_eq!(p.sanitize_filename(".."), "attachment");
        assert_eq!(p.sanitize_filename(""), "attachment");
        assert_eq!(p.sanitize_filename("report.pdf"), "report.pdf");
        assert_eq!(p.sanitize_filename("a\u{0}b"), "a_b");
    }

    #[test]
    fn threshold_behavior() {
        let p = AttachmentPolicy::default();
        assert!(p.should_return_inline(DEFAULT_DIRECT_THRESHOLD));
        assert!(!p.should_return_inline(DEFAULT_DIRECT_THRESHOLD + 1));
    }

    #[test]
    fn temp_file_stays_in_cache_dir() {
        let dir = std::env::temp_dir().join(format!("gmail-mcp-test-{}", std::process::id()));
        let p = AttachmentPolicy {
            cache_dir: dir.clone(),
            ..Default::default()
        };
        let path = p.write_temp("../../evil.txt", b"data").unwrap();
        assert!(p.is_within_cache(&path));
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"data");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
