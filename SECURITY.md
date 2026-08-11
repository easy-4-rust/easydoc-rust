# Security Policy

## Supported Versions

| Version | Supported | Until |
|---------|-----------|-------|
| 0.1.0-alpha.x | Yes | Next stable release (0.1.0) |
| < 0.1.0 | No | -- |

During the alpha phase, only the latest alpha release receives security fixes.
No LTS or multi-version support is provided before 0.1.0 stable.

## Reporting a Vulnerability

### Preferred: GitHub Security Advisories

Use GitHub's private vulnerability reporting to create a security advisory:

**https://github.com/easy-4-rust/easydoc-rust/security/advisories/new**

This is a confidential channel. Only repository maintainers and the reporter
can see the advisory until it is published. You will receive an acknowledgement
within 48 hours.

### Alternative: Email

For sensitive reports that cannot use GitHub Security Advisories:

**security@easydoc-rust.example.com**

> Note: This is a placeholder address. Check the repository for the current
> contact email before sending.

### Non-Sensitive Bugs

For issues that are **not** security-sensitive (typo, build failure, feature
request), use the public issue tracker:

**https://github.com/easy-4-rust/easydoc-rust/issues/new**

Do **not** report security vulnerabilities through public issues.

### What to Include

A good vulnerability report should contain:

- **Description** -- what is the vulnerability and how it can be exploited
- **Reproduction steps** -- minimal test case or sequence of inputs
- **Affected versions** -- which releases and platforms are impacted
- **Known mitigations** -- any workarounds or configuration changes that reduce risk
- **Severity assessment** -- your estimate (Critical / High / Medium / Low)

## Response Timeline

| Stage | Commitment |
|-------|-----------|
| Acknowledgement | Within 48 hours |
| Severity triage | Within 7 days |
| Patch release (Critical) | Within 14 days |
| Patch release (High) | Within 30 days |
| Patch release (Medium/Low) | Next regular release |

If a fix cannot meet these timelines, we will communicate the revised schedule
through the advisory.

## Security Update Channels

Security fixes are announced through:

- **GitHub Security Advisories** -- primary channel, confidential until disclosure
- **[CHANGELOG.md](CHANGELOG.md)** -- all security-related changes are noted
- **GitHub Releases** -- patched versions are published with release notes

To receive release notifications only, configure your watch settings:

**Watch** > **Custom** > check **Releases**

## Deployed Security Measures

This project applies the following security practices:

### Compile-Time Enforcement

- `#![deny(unsafe_code)]` across all 9 workspace crates -- no `unsafe` blocks
  in production code

### Dependency Auditing

- **cargo-audit** -- RustSec advisory database checks (CI + weekly schedule)
- **cargo-deny** -- license allowlist, banned crates, source restrictions
  (crates.io only), advisory integration

### Input Hardening

| Threat | Mitigation |
|--------|-----------|
| ZIP bomb | Max expanded size 1 GB, max compression ratio 1,000:1 |
| Zip Slip | Path traversal check (`..` and absolute paths rejected) |
| SSRF | Default deny for localhost, RFC1918, link-local, carrier-grade NAT, IPv6 ULA; DNS re-check |
| Path traversal (MCP) | `canonicalize` + `starts_with` boundary check |

### Build Configuration

- **MSRV**: Rust 1.88.0 (declared in `rust-toolchain.toml`)
- **License**: Apache-2.0 (enforced via cargo-deny allowlist)
- **Clippy**: pedantic lint group enabled, `-D warnings` in CI
- **Formatting**: `cargo fmt --check` enforced in CI

## Acknowledgements

We credit security researchers who follow responsible disclosure. If you report
a confirmed vulnerability, we will add your name here (with your permission).

- (none yet)

## References

- [Rust Security Advisory Database (RustSec)](https://rustsec.org/)
- [cargo-audit documentation](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [cargo-deny documentation](https://github.com/EmbarkStudios/cargo-deny)
- [GitHub Security Advisories](https://docs.github.com/en/code-security/security-advisories)

<!-- A Chinese translation (SECURITY_zh.md) is planned. -->
