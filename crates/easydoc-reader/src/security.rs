//! Security guards for SSRF protection and ZIP bomb / element explosion prevention.
//!
//! Mirrors the protection offered by `OfficeCLI`'s `SsrfGuard` and
//! `GuardDecompressionBomb` / `GuardElementExplosion`.
//!
//! These guards should be called at the reader entry point (`from_path` / `from_reader`)
//! and before ZIP content extraction.

use std::net::{Ipv4Addr, Ipv6Addr};

// ---------------------------------------------------------------------------
// SsrfGuard
// ---------------------------------------------------------------------------

/// SSRF (Server-Side Request Forgery) protection configuration.
///
/// Validates hyperlinks extracted from DOCX documents against a set of
/// allowed schemes, blocked hosts, and optionally resolves DNS to check
/// for private / loopback IP addresses.
///
/// # Examples
///
/// ```
/// use easydoc_reader::security::SsrfGuard;
///
/// let guard = SsrfGuard::new();
/// assert!(guard.check_url("https://example.com").is_ok());
/// assert!(guard.check_url("http://127.0.0.1/admin").is_err());
/// assert!(guard.check_url("ftp://example.com").is_err());
/// ```
#[derive(Debug, Clone)]
pub struct SsrfGuard {
    /// Allowed URI schemes (lowercase). Default: `["http", "https", "mailto"]`.
    pub allowed_schemes: Vec<String>,
    /// Blocked host names (lowercase). Default: `["localhost"]`.
    pub blocked_hosts: Vec<String>,
    /// Whether to resolve DNS names and check the resulting IP addresses
    /// against private / loopback ranges. Default: `true`.
    pub resolve_dns: bool,
    /// Whether to check IP addresses against private / loopback / link-local
    /// ranges. Default: `true`. Set to `false` in permissive mode.
    pub check_private_ips: bool,
}

impl Default for SsrfGuard {
    fn default() -> Self {
        Self {
            allowed_schemes: vec!["http".to_owned(), "https".to_owned(), "mailto".to_owned()],
            blocked_hosts: vec!["localhost".to_owned()],
            resolve_dns: true,
            check_private_ips: true,
        }
    }
}

impl SsrfGuard {
    /// Creates a guard with the default conservative policy.
    ///
    /// Allows `http`, `https`, `mailto` schemes; blocks `localhost` and all
    /// private / loopback IP ranges; resolves DNS to verify host names.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a permissive guard that only enforces scheme restrictions.
    ///
    /// No hosts are blocked, DNS resolution is disabled, and private IP
    /// ranges are not checked. Useful when the caller only needs basic
    /// scheme validation.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            allowed_schemes: vec!["http".to_owned(), "https".to_owned(), "mailto".to_owned()],
            blocked_hosts: Vec::new(),
            resolve_dns: false,
            check_private_ips: false,
        }
    }

    /// Checks whether the given URL passes all SSRF protection rules.
    ///
    /// # Errors
    ///
    /// Returns a human-readable error message describing the violation.
    pub fn check_url(&self, url: &str) -> Result<(), String> {
        // Split on first ':' to extract the scheme. This handles both
        // "https://host/path" and "mailto:user@example.com" forms.
        let (scheme, rest) = url
            .split_once(':')
            .ok_or_else(|| "missing scheme".to_owned())?;

        let scheme_lower = scheme.to_ascii_lowercase();
        if !self.allowed_schemes.iter().any(|s| s == &scheme_lower) {
            return Err(format!("scheme '{scheme}' not allowed"));
        }

        // mailto: URLs do not have a network-reachable host.
        if scheme_lower == "mailto" {
            return Ok(());
        }

        // For http/https, rest starts with "//host/path".
        // Strip the leading "//" if present.
        let rest = rest.strip_prefix("//").unwrap_or(rest);

        let host_raw = rest.split('/').next().unwrap_or(rest);
        // For IPv6, the host may be enclosed in brackets: [::1]:8080
        let host = if host_raw.starts_with('[') {
            host_raw
                .split(']')
                .next()
                .and_then(|h| h.strip_prefix('['))
                .unwrap_or(host_raw)
        } else {
            host_raw.split(':').next().unwrap_or(host_raw)
        };
        if host.is_empty() {
            return Err("empty host".to_owned());
        }

        let host_lower = host.to_ascii_lowercase();
        if self.blocked_hosts.iter().any(|h| h == &host_lower) {
            return Err(format!("blocked host: {host}"));
        }

        if self.check_private_ips {
            if let Ok(v4) = host.parse::<Ipv4Addr>()
                && Self::is_blocked_ipv4(v4)
            {
                return Err(format!("blocked IPv4: {v4}"));
            }
            if let Ok(v6) = host.parse::<Ipv6Addr>()
                && Self::is_blocked_ipv6(v6)
            {
                return Err(format!("blocked IPv6: {v6}"));
            }

            if self.resolve_dns {
                // Resolve the host name and verify each resulting IP.
                use std::net::ToSocketAddrs;
                if let Ok(addrs) = (host, 0u16).to_socket_addrs() {
                    for addr in addrs {
                        match addr.ip() {
                            std::net::IpAddr::V4(v4) => {
                                if Self::is_blocked_ipv4(v4) {
                                    return Err(format!("DNS resolved to blocked IPv4: {v4}"));
                                }
                            }
                            std::net::IpAddr::V6(v6) => {
                                if Self::is_blocked_ipv6(v6) {
                                    return Err(format!("DNS resolved to blocked IPv6: {v6}"));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns `true` if the IPv4 address falls into a blocked range.
    ///
    /// Blocked ranges:
    /// - `127.0.0.0/8` (loopback)
    /// - `10.0.0.0/8` (private)
    /// - `172.16.0.0/12` (private)
    /// - `192.168.0.0/16` (private)
    /// - `169.254.0.0/16` (link-local)
    /// - `100.64.0.0/10` (carrier-grade NAT)
    /// - `0.0.0.0/8` (reserved)
    fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
        let o = ip.octets();
        o[0] == 127                                          // 127.0.0.0/8
            || o[0] == 10                                    // 10.0.0.0/8
            || (o[0] == 172 && (16..=31).contains(&o[1]))   // 172.16.0.0/12
            || (o[0] == 192 && o[1] == 168)                 // 192.168.0.0/16
            || (o[0] == 169 && o[1] == 254)                 // 169.254.0.0/16 link-local
            || (o[0] == 100 && (64..=127).contains(&o[1]))  // 100.64.0.0/10 CGN
            || o[0] == 0 // 0.0.0.0/8
    }

    /// Returns `true` if the IPv6 address falls into a blocked range.
    ///
    /// Blocked ranges:
    /// - Loopback (`::1`)
    /// - Unspecified (`::`)
    /// - Unique-local (`fc00::/7`)
    /// - Link-local (`fe80::/10`)
    /// - Multicast (`ff00::/8`)
    fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
        if ip.is_loopback() || ip.is_unspecified() {
            return true;
        }
        let s = ip.segments();
        (s[0] & 0xfe00) == 0xfc00   // fc00::/7 unique-local
            || (s[0] & 0xffc0) == 0xfe80 // fe80::/10 link-local
            || (s[0] & 0xff00) == 0xff00 // ff00::/8 multicast
    }
}

// ---------------------------------------------------------------------------
// PackageLimits
// ---------------------------------------------------------------------------

/// ZIP archive size and complexity limits to prevent decompression bombs
/// and element explosion attacks.
///
/// # Examples
///
/// ```
/// use easydoc_reader::security::PackageLimits;
///
/// let limits = PackageLimits::new();
/// // Default: 100 MB total, 50 MB per entry, 100x ratio, 10 000 entries
/// assert_eq!(limits.max_total_uncompressed, 100 * 1024 * 1024);
/// ```
#[derive(Debug, Clone)]
pub struct PackageLimits {
    /// Maximum total uncompressed size across all entries (bytes).
    /// Default: 100 MB.
    pub max_total_uncompressed: u64,
    /// Maximum uncompressed size for a single entry (bytes).
    /// Default: 50 MB.
    pub max_single_uncompressed: u64,
    /// Maximum allowed compression ratio (uncompressed / compressed).
    /// Default: 100.
    pub max_compression_ratio: u64,
    /// Maximum number of entries in the archive.
    /// Default: 10 000.
    pub max_entries: usize,
    /// Maximum length of a single filename (bytes).
    /// Default: 256.
    pub max_filename_len: usize,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_total_uncompressed: 100 * 1024 * 1024, // 100 MB
            max_single_uncompressed: 50 * 1024 * 1024, // 50 MB
            max_compression_ratio: 100,                // 100x
            max_entries: 10_000,
            max_filename_len: 256,
        }
    }
}

impl PackageLimits {
    /// Creates limits with the default conservative policy.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates a ZIP archive against all configured limits.
    ///
    /// Checks:
    /// 1. Entry count does not exceed `max_entries`.
    /// 2. Each filename is within `max_filename_len`.
    /// 3. No path traversal (`..` or leading `/`) -- Zip Slip prevention.
    /// 4. No single entry exceeds `max_single_uncompressed`.
    /// 5. Per-entry compression ratio does not exceed `max_compression_ratio`.
    /// 6. Total uncompressed size does not exceed `max_total_uncompressed`.
    /// 7. Overall compression ratio does not exceed `max_compression_ratio`.
    ///
    /// # Errors
    ///
    /// Returns a human-readable error describing which limit was exceeded.
    pub fn validate_archive<R: std::io::Read + std::io::Seek>(
        &self,
        archive: &mut zip::ZipArchive<R>,
    ) -> Result<(), String> {
        let len = archive.len();
        if len > self.max_entries {
            return Err(format!("too many entries: {len} > {}", self.max_entries));
        }

        let mut total_uncompressed: u64 = 0;
        let mut total_compressed: u64 = 0;

        for i in 0..len {
            let entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name();

            if name.len() > self.max_filename_len {
                return Err(format!(
                    "filename too long: {} bytes (max {})",
                    name.len(),
                    self.max_filename_len
                ));
            }

            // Zip Slip: reject path traversal and absolute paths.
            if name.contains("..") || name.starts_with('/') {
                return Err(format!("suspicious path: {name}"));
            }

            let size = entry.size();
            let compressed = entry.compressed_size();

            if size > self.max_single_uncompressed {
                return Err(format!(
                    "entry '{name}' too large: {size} bytes (max {})",
                    self.max_single_uncompressed
                ));
            }

            if compressed > 0 && size / compressed > self.max_compression_ratio {
                return Err(format!(
                    "entry '{name}' compression ratio too high: {}x (max {}x)",
                    size / compressed,
                    self.max_compression_ratio
                ));
            }

            total_uncompressed = total_uncompressed.saturating_add(size);
            total_compressed = total_compressed.saturating_add(compressed);
        }

        if total_uncompressed > self.max_total_uncompressed {
            return Err(format!(
                "total uncompressed too large: {total_uncompressed} bytes (max {})",
                self.max_total_uncompressed
            ));
        }

        if total_compressed > 0
            && total_uncompressed / total_compressed > self.max_compression_ratio
        {
            return Err(format!(
                "overall compression ratio too high: {}x (max {}x)",
                total_uncompressed / total_compressed,
                self.max_compression_ratio
            ));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SecurityPolicy
// ---------------------------------------------------------------------------

/// Combined security policy holding SSRF and ZIP limits guards.
///
/// Passed into the reader to enforce security checks at the entry point.
///
/// # Examples
///
/// ```
/// use easydoc_reader::security::SecurityPolicy;
///
/// let policy = SecurityPolicy::new();
/// assert!(policy.ssrf.check_url("https://example.com").is_ok());
/// assert!(policy.ssrf.check_url("http://127.0.0.1").is_err());
/// ```
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// SSRF protection guard.
    pub ssrf: SsrfGuard,
    /// ZIP bomb / element explosion limits.
    pub limits: PackageLimits,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            ssrf: SsrfGuard::new(),
            limits: PackageLimits::new(),
        }
    }
}

impl SecurityPolicy {
    /// Creates a policy with the default conservative settings.
    ///
    /// SSRF guard blocks private IPs and localhost; ZIP limits cap at
    /// 100 MB total / 50 MB per entry / 100x compression ratio.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a permissive policy: only scheme enforcement for SSRF,
    /// and no ZIP limits (validation will always pass).
    ///
    /// Useful for trusted input environments where only basic sanity
    /// checks are desired.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            ssrf: SsrfGuard::permissive(),
            // No limits: max everything out.
            limits: PackageLimits {
                max_total_uncompressed: u64::MAX,
                max_single_uncompressed: u64::MAX,
                max_compression_ratio: u64::MAX,
                max_entries: usize::MAX,
                max_filename_len: usize::MAX,
            },
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SsrfGuard tests
    // -----------------------------------------------------------------------

    #[test]
    fn check_url_blocks_localhost_literal() {
        let guard = SsrfGuard::new();
        let err = guard.check_url("http://localhost/admin").unwrap_err();
        assert!(err.contains("blocked host"), "error: {err}");
    }

    #[test]
    fn check_url_blocks_private_ip_10_x() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard.check_url("http://10.0.0.1/secret").unwrap_err();
        assert!(err.contains("blocked IPv4"), "error: {err}");
    }

    #[test]
    fn check_url_blocks_private_ip_192_168() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard.check_url("http://192.168.1.1/internal").unwrap_err();
        assert!(err.contains("blocked IPv4"), "error: {err}");
    }

    #[test]
    fn check_url_blocks_private_ip_172_16() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard.check_url("http://172.16.0.1/api").unwrap_err();
        assert!(err.contains("blocked IPv4"), "error: {err}");
    }

    #[test]
    fn check_url_blocks_ipv6_loopback() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard.check_url("http://[::1]/admin").unwrap_err();
        assert!(
            err.contains("blocked IPv6") || err.contains("blocked host"),
            "error: {err}"
        );
    }

    #[test]
    fn check_url_blocks_link_local_169_254() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard
            .check_url("http://169.254.169.254/metadata")
            .unwrap_err();
        assert!(err.contains("blocked IPv4"), "error: {err}");
    }

    #[test]
    fn check_url_rejects_ftp_scheme() {
        let guard = SsrfGuard::new();
        let err = guard.check_url("ftp://example.com/file.txt").unwrap_err();
        assert!(err.contains("scheme"), "error: {err}");
    }

    #[test]
    fn check_url_rejects_empty_host() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard.check_url("http:///path").unwrap_err();
        assert!(err.contains("empty host"), "error: {err}");
    }

    #[test]
    fn check_url_rejects_missing_scheme() {
        let guard = SsrfGuard::new();
        let err = guard.check_url("example.com/page").unwrap_err();
        assert!(err.contains("missing scheme"), "error: {err}");
    }

    #[test]
    fn check_url_allows_https_external() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        guard
            .check_url("https://example.com/page")
            .expect("https://example.com should be allowed");
    }

    #[test]
    fn check_url_allows_mailto() {
        let guard = SsrfGuard::new();
        guard
            .check_url("mailto:user@example.com")
            .expect("mailto: should be allowed");
    }

    #[test]
    fn check_url_blocks_carrier_grade_nat() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard.check_url("http://100.64.0.1/internal").unwrap_err();
        assert!(err.contains("blocked IPv4"), "error: {err}");
    }

    #[test]
    fn check_url_blocks_zero_network() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        let err = guard.check_url("http://0.0.0.0/admin").unwrap_err();
        assert!(err.contains("blocked IPv4"), "error: {err}");
    }

    #[test]
    fn permissive_guard_allows_private_ip() {
        let guard = SsrfGuard::permissive();
        guard
            .check_url("http://192.168.1.1/api")
            .expect("permissive guard should allow private IPs");
    }

    #[test]
    fn permissive_guard_still_blocks_unknown_scheme() {
        let guard = SsrfGuard::permissive();
        let err = guard.check_url("ftp://example.com").unwrap_err();
        assert!(err.contains("scheme"), "error: {err}");
    }

    #[test]
    fn check_url_with_port() {
        let guard = SsrfGuard {
            resolve_dns: false,
            ..SsrfGuard::new()
        };
        guard
            .check_url("https://example.com:8080/api")
            .expect("external host with port should be allowed");
    }

    // -----------------------------------------------------------------------
    // PackageLimits tests
    // -----------------------------------------------------------------------

    /// Builds a minimal in-memory ZIP archive with the given entries.
    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let w = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(w);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (name, data) in entries {
                zip.start_file(*name, options).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn validate_archive_passes_normal() {
        let data = build_zip(&[
            ("word/document.xml", b"<document/>"),
            ("word/styles.xml", b"<styles/>"),
        ]);
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        let limits = PackageLimits::new();
        limits
            .validate_archive(&mut archive)
            .expect("normal archive should pass");
    }

    #[test]
    fn validate_archive_rejects_too_many_entries() {
        let mut entries: Vec<(&str, Vec<u8>)> = Vec::new();
        for i in 0..50 {
            entries.push((
                // Leak a static-ish string for the test.
                Box::leak(format!("file{i}.txt").into_boxed_str()) as &str,
                b"hello".to_vec(),
            ));
        }
        let entry_refs: Vec<(&str, &[u8])> =
            entries.iter().map(|(n, d)| (*n, d.as_slice())).collect();
        let data = build_zip(&entry_refs);
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();

        let limits = PackageLimits {
            max_entries: 10,
            ..PackageLimits::new()
        };
        let err = limits.validate_archive(&mut archive).unwrap_err();
        assert!(err.contains("too many entries"), "error: {err}");
    }

    #[test]
    fn validate_archive_rejects_zip_slip() {
        let data = build_zip(&[("word/../../../etc/passwd", b"root:x:0:0")]);
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        let limits = PackageLimits::new();
        let err = limits.validate_archive(&mut archive).unwrap_err();
        assert!(err.contains("suspicious path"), "error: {err}");
    }

    #[test]
    fn validate_archive_rejects_absolute_path() {
        let data = build_zip(&[("/etc/passwd", b"root:x:0:0")]);
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        let limits = PackageLimits::new();
        let err = limits.validate_archive(&mut archive).unwrap_err();
        assert!(err.contains("suspicious path"), "error: {err}");
    }

    #[test]
    fn validate_archive_rejects_large_entry() {
        // 200 bytes of data, limit set to 100 bytes.
        let big_data = vec![0u8; 200];
        let data = build_zip(&[("big.bin", big_data.as_slice())]);
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();

        let limits = PackageLimits {
            max_single_uncompressed: 100,
            ..PackageLimits::new()
        };
        let err = limits.validate_archive(&mut archive).unwrap_err();
        assert!(err.contains("too large"), "error: {err}");
    }

    #[test]
    fn validate_archive_rejects_high_compression_ratio() {
        // Create a ZIP entry with Deflated compression to get a real ratio.
        use std::io::Write;
        let big_data = vec![0u8; 100_000];
        let mut buf = Vec::new();
        {
            let w = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(w);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(9));
            zip.start_file("bomb.bin", options).unwrap();
            zip.write_all(&big_data).unwrap();
            zip.finish().unwrap();
        }

        let reader = std::io::Cursor::new(buf);
        let mut archive = zip::ZipArchive::new(reader).unwrap();

        // Set ratio limit to 2x -- highly compressible data will exceed this.
        let limits = PackageLimits {
            max_compression_ratio: 2,
            ..PackageLimits::new()
        };
        let err = limits.validate_archive(&mut archive).unwrap_err();
        assert!(err.contains("compression ratio too high"), "error: {err}");
    }

    #[test]
    fn validate_archive_rejects_filename_too_long() {
        let long_name = format!("{}.xml", "a".repeat(300));
        let name_ref: &str = Box::leak(long_name.into_boxed_str());
        let data = build_zip(&[(name_ref, b"<data/>")]);
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();

        let limits = PackageLimits {
            max_filename_len: 100,
            ..PackageLimits::new()
        };
        let err = limits.validate_archive(&mut archive).unwrap_err();
        assert!(err.contains("filename too long"), "error: {err}");
    }

    #[test]
    fn validate_archive_rejects_total_too_large() {
        let a = vec![0u8; 60];
        let b = vec![0u8; 60];
        let data = build_zip(&[("a.bin", a.as_slice()), ("b.bin", b.as_slice())]);
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader).unwrap();

        let limits = PackageLimits {
            max_total_uncompressed: 100,
            ..PackageLimits::new()
        };
        let err = limits.validate_archive(&mut archive).unwrap_err();
        assert!(err.contains("total uncompressed too large"), "error: {err}");
    }

    // -----------------------------------------------------------------------
    // SecurityPolicy tests
    // -----------------------------------------------------------------------

    #[test]
    fn security_policy_default_is_conservative() {
        let policy = SecurityPolicy::new();
        // SSRF blocks localhost
        assert!(policy.ssrf.check_url("http://localhost/x").is_err());
        // Limits are reasonable
        assert_eq!(policy.limits.max_total_uncompressed, 100 * 1024 * 1024);
    }

    #[test]
    fn security_policy_permissive_relaxes_limits() {
        let policy = SecurityPolicy::permissive();
        // SSRF permissive still blocks unknown schemes
        assert!(policy.ssrf.check_url("ftp://x.com").is_err());
        // But allows private IPs
        assert!(policy.ssrf.check_url("http://10.0.0.1/x").is_ok());
        // ZIP limits are maximized
        assert_eq!(policy.limits.max_entries, usize::MAX);
    }
}
