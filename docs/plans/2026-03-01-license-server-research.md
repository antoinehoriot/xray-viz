# License Validation Server Research

**Date:** 2026-03-01
**Task:** codebase-viz-1ae1
**Status:** Research complete

---

## 1. Current State

The xray CLI uses **offline-only** license validation:

- **Key format:** `XRAY-XXXX-XXXX` (prefix + two 4-char alphanumeric segments)
- **Storage:** `~/.xray/license.key` (plaintext file)
- **Validation:** Pure format check via `is_valid_key()` in `crates/xray-cli/src/license.rs`
- **Gate:** `is_pro()` returns true if a valid-format key exists on disk
- **Frontend:** `localStorage('xray_pro_key')` mirrors the gate in the web UI
- **Commands:** `xray license activate <key>`, `xray license status`, `xray license deactivate`

### Limitations

- **No server validation** — any string matching `XRAY-XXXX-XXXX` works
- **No expiry** — keys are valid forever once stored
- **No seat limits** — same key works on unlimited machines
- **No revocation** — no way to disable a leaked key
- **No usage tracking** — no visibility into active installations
- **Trivially bypassed** — users can craft valid keys without purchasing

---

## 2. SaaS Options

### 2.1 Keygen.sh (Recommended for dedicated licensing)

**What it is:** Purpose-built software licensing and distribution API.

**Pricing:**
- **Dev (Free):** 100 ALUs (active licensed users within 90 days), 10 releases
- **Std tiers (Paid):** Scaling ALU limits, Distribution API, custom environments, roles/permissions, 8x5 support (24hr response). Pricing is slider-based on volume.
- **Ent tiers:** Audit logs, SAML/SSO, IP allow/deny lists, custom rate limits, 99.99% SLA, 12x5 support (8hr response)
- **Add-ons:** Whitelabel ($995/yr/domain), Premium SLA ($995/mo)

**Key features:**
- License key generation, validation, activation, deactivation
- Policy-based licensing (node-locked, floating, timed trials, feature-based)
- Machine activation with device fingerprinting
- Ed25519 cryptographic offline validation (signed license files)
- Air-gapped activation via QR codes
- Distribution API (OCI/Docker, npm, PyPI, binary releases)
- SOC 2 Type II compliant (Jan 2026)
- ECDSA P-256 for FIPS 140-3 compliance

**Rust integration:**
- Unofficial SDK: keygen-rs (https://crates.io/crates/keygen-rs)
- Supports cloud and self-hosted API endpoints
- License validation, machine activation, heartbeat monitoring
- Admin APIs for CRUD operations

**Self-hosting:**
- **Keygen CE** (Community Edition): Free, self-hosted via Docker
- **Keygen EE** (Enterprise Edition): Requires license, adds audit logs, permissions, environments
- Source available on GitHub: keygen-sh/keygen-api

**Pros:**
- Purpose-built for exactly this use case
- Excellent offline/cryptographic validation support
- Rust SDK exists
- Self-hosting option removes vendor lock-in
- Comprehensive device activation and seat management

**Cons:**
- Another service to manage (unless self-hosted)
- Pricing can scale with user count
- Unofficial Rust SDK (not maintained by Keygen)

**Fit for xray:** **Excellent.** Covers all requirements (key generation, expiry, seats, revocation). Cryptographic offline validation aligns with CLI tool use case. Self-hosted CE eliminates vendor dependency.

---

### 2.2 Lemon Squeezy (Best for payments + licensing bundle)

**What it is:** Merchant of record platform with built-in license key management.

**Pricing:**
- **5% + $0.50 per transaction** (all-inclusive, no monthly fee)
- Includes tax compliance, fraud protection, payment methods
- Email marketing: free up to 500 subscribers, then $10/mo per 1k

**Key features:**
- Automatic license key generation on purchase
- License activation/deactivation/validation via API
- Configurable key length and activation limits
- Per-instance activation tracking
- Rate limited to 60 requests/minute

**Pros:**
- Zero monthly cost — pay only on transactions
- Handles payments, tax, and licensing in one platform
- Simple API for basic license operations
- Merchant of record reduces legal/tax burden

**Cons:**
- **No Rust SDK** — must use HTTP API directly
- 5% per transaction is significant at scale
- **No offline/cryptographic validation** — requires internet for validation
- Limited licensing models (no floating, no feature-based)
- 60 req/min rate limit could be restrictive
- No self-hosting option

**Fit for xray:** **Good for MVP.** Handles payments and basic licensing. Lacks offline validation (critical for CLI tools) and advanced licensing models.

---

### 2.3 Paddle (Best for SaaS/subscription billing)

**What it is:** Payment infrastructure and merchant of record platform.

**Pricing:**
- **5% + $0.50 per transaction** (no monthly fee)
- Includes tax compliance, chargeback coverage, fraud protection, customer support

**Key features:**
- License generation and distribution
- Usage monitoring and revocation
- Subscription management with churn prevention
- Go and Python SDKs (no Rust SDK)

**Pros:**
- Comprehensive payment infrastructure
- Tax compliance as merchant of record

**Cons:**
- **Not licensing-focused** — license management is secondary to billing
- **No Rust SDK**
- No offline validation
- No cryptographic license files
- No device fingerprinting or machine activation

**Fit for xray:** **Poor for licensing, good for billing.** Could pair with Keygen (Keygen has a Paddle integration). Not suitable as standalone license solution.

---

### 2.4 Gumroad (Simplest option)

**What it is:** Creator-focused e-commerce platform with basic license keys.

**Pricing:**
- **10% flat fee per transaction** (no monthly cost)

**Key features:**
- License key generation on purchase
- Verification endpoint: POST https://api.gumroad.com/v2/licenses/verify
- Parameters: product_id, license_key, optional increment_uses_count
- Usage counting (per-verification increment)

**Pros:**
- Extremely simple integration (single API endpoint)
- No monthly costs

**Cons:**
- **10% fee is highest** of all options
- **No Rust SDK** — HTTP only
- **No offline validation**
- No device activation or machine binding
- No seat management or expiry management
- Limited API

**Fit for xray:** **Poor.** Too limited for production licensing. Acceptable only for quick proof-of-concept.

---

## 3. Custom Rust Server Option

### Architecture

Build a license validation server using the existing xray tech stack:

```
+---------------+     HTTPS      +--------------------+     +----------+
|  xray CLI     | -------------> |  License Server    | --> | SQLite / |
|  (client)     | <------------- |  (axum 0.7)        | <-- | Postgres |
+---------------+  signed JSON   +--------------------+     +----------+
```

### Implementation sketch

**Server (axum 0.7):**
- POST /api/v1/license/validate — validate key, return signed response
- POST /api/v1/license/activate — activate on a machine (device fingerprint)
- POST /api/v1/license/deactivate — release a seat
- GET /api/v1/license/status — check key status, expiry, seats
- JWT or Ed25519-signed responses for offline caching

**Database schema:**
```sql
CREATE TABLE licenses (
    id TEXT PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    email TEXT NOT NULL,
    plan TEXT NOT NULL,
    max_seats INTEGER DEFAULT 1,
    expires_at TIMESTAMP,
    revoked_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE activations (
    id TEXT PRIMARY KEY,
    license_id TEXT REFERENCES licenses(id),
    machine_fingerprint TEXT NOT NULL,
    hostname TEXT,
    os TEXT,
    last_seen_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(license_id, machine_fingerprint)
);
```

**Rust crates:**
- axum 0.7 — HTTP server (already used in xray-cli)
- jsonwebtoken or ed25519-dalek — signed responses
- sqlx — database access
- blake3 — machine fingerprinting (already in workspace)
- serde / serde_json — serialization (already in workspace)

**Pros:**
- Full control over licensing logic
- No per-transaction fees
- No vendor dependency
- Can leverage existing axum/Rust expertise
- Offline validation via signed license files

**Cons:**
- Significant development effort (2-4 weeks)
- Must handle infrastructure (hosting, monitoring, backups)
- Must build admin dashboard
- Must handle security (key generation entropy, rate limiting, DDoS)
- No payment integration — need separate payment provider

**Fit for xray:** **Good for long-term, high upfront cost.** Makes sense if licensing requirements are complex or if avoiding ongoing SaaS fees is important.

---

## 4. Architecture Patterns

### 4.1 Online Validation with Grace Period (Recommended)

```
Startup -> Try online validation -> Success? Cache response + timestamp
                                 -> Fail?   Check cache age
                                            -> < grace period? Allow (warn)
                                            -> > grace period? Block (error)
```

**How it works:**
- On each launch, CLI attempts to validate license against server
- Successful validation caches a signed response locally (~/.xray/license.cache)
- If network is unavailable, cached response is used within a grace period (7-30 days)
- After grace period expires without successful validation, features are disabled
- Warning messages appear 3-7 days before grace period expiry

**Grace period strategy:**

| Phase | Days since last validation | Behavior |
|-------|---------------------------|----------|
| Valid | 0-14 | Full functionality, silent |
| Warning | 15-21 | Full functionality, warning on startup |
| Critical | 22-28 | Full functionality, warning on every command |
| Expired | 29+ | Downgrade to Community edition |

**Pros:**
- Works offline for reasonable periods
- Server retains control over revocation and expiry
- Simple to implement

**Cons:**
- Requires eventual internet access
- Must handle clock manipulation (cache signed server timestamps)

### 4.2 Cryptographic Offline Validation (Ed25519 Signed Licenses)

```
Purchase -> Server generates signed license file (Ed25519)
         -> User receives license file or encoded key
CLI      -> Decode key -> Verify signature with embedded public key
         -> Check expiry in signed payload -> Allow/deny
```

**How it works:**
- License key is a signed data payload containing: plan, expiry date, seat count, machine fingerprint
- Public key is embedded in the xray binary
- Validation is purely local — no network required
- Expiry is encoded in the signed payload and cannot be tampered with

**Key format evolution:**
```
Current:  XRAY-AB12-CD34              (format-only check)
Proposed: XRAY.eyJwbGFuIjoicHJvI...   (base64-encoded signed JWT)
```

**Implementation:**
```rust
use ed25519_dalek::{PublicKey, Signature, Verifier};

struct LicensePayload {
    plan: String,
    email: String,
    expires_at: i64,
    max_seats: u32,
    machine_id: Option<String>,
}

fn validate_offline(license_key: &str, public_key: &PublicKey) -> Result<LicensePayload> {
    let (payload_bytes, signature_bytes) = decode_key(license_key)?;
    let signature = Signature::from_bytes(&signature_bytes)?;
    public_key.verify(&payload_bytes, &signature)?;
    let payload: LicensePayload = serde_json::from_slice(&payload_bytes)?;
    if payload.expires_at < now_unix() {
        return Err(LicenseError::Expired);
    }
    Ok(payload)
}
```

**Pros:**
- Fully offline — no server needed after key generation
- Tamper-proof — signed payload cannot be modified
- Fast — no network latency
- Air-gapped environment friendly

**Cons:**
- Cannot revoke keys without a blocklist (which needs updates)
- Key rotation requires new binary builds
- Longer license keys (base64-encoded payloads)

### 4.3 Hybrid (Online + Cryptographic Offline Fallback)

Combines online validation (4.1) with cryptographic caching (4.2):
- Online validation returns a signed license file (with server-controlled expiry)
- When offline, the cached signed file is verified cryptographically
- Server can issue short-lived license files (e.g., 30-day validity)
- Revocation works through short expiry — revoked keys simply don't get renewed

**Pros:** Best of both worlds — online control + offline resilience
**Cons:** Most complex to implement, two validation paths

### 4.4 Hardware Fingerprinting / Machine Binding

**Purpose:** Limit license to specific machines (seat enforcement).

**Fingerprint components (cross-platform):**
- Machine GUID (Windows registry, macOS IOPlatformUUID, Linux /etc/machine-id)
- CPU model + core count
- Total RAM

**Rust implementation:**
```rust
use blake3;

fn machine_fingerprint() -> String {
    let mut data = String::new();
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output();
        if let Ok(out) = output {
            data.push_str(&String::from_utf8_lossy(&out.stdout));
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            data.push_str(&id);
        }
    }
    blake3::hash(data.as_bytes()).to_hex()[..16].to_string()
}
```

**Existing crate:** device-fingerprint (Windows-only). For cross-platform, a custom implementation using blake3 (already in workspace) is preferable.

**Pros:** Enforces seat limits, prevents casual key sharing, uses blake3 already in workspace
**Cons:** Hardware changes invalidate fingerprint, VMs can spoof, adds activation friction

---

## 5. Recommendation

### Phase 1 — Quick Win (1-2 days)

**Use Keygen.sh Cloud (free Dev tier)** for immediate server-side validation:
1. Create Keygen account, define a Pro policy with seat limits and expiry
2. Integrate keygen-rs crate into xray-cli for online validation
3. Add grace period caching (Pattern 4.1) for offline resilience
4. Keep backward compatibility with existing XRAY-XXXX-XXXX format during migration

**Why:** Gets production licensing with minimal effort. Free tier supports 100 ALUs which is sufficient for early adoption. Upgradeable to self-hosted later.

### Phase 2 — Production Hardening (1-2 weeks)

1. Add machine fingerprinting for seat enforcement (Pattern 4.4)
2. Implement cryptographic offline validation (Pattern 4.2) using Keygen Ed25519 signed license files
3. Add xray license activate --offline for air-gapped environments
4. Integrate with payment provider (Lemon Squeezy or Paddle) for purchase flow

### Phase 3 — Self-Hosted (if needed)

If vendor dependency becomes a concern or ALU costs grow:
1. Deploy Keygen CE (self-hosted, free) on own infrastructure
2. Point keygen-rs to self-hosted endpoint (single config change)
3. Full control, zero per-user fees

### Alternative: Custom Server

If requirements are highly custom or Keygen doesn't fit:
1. Build axum-based license server (see Section 3)
2. Use ed25519-dalek for signed license files
3. Integrate with Stripe/Paddle for payments
4. Estimated effort: 2-4 weeks including admin dashboard

---

## 6. Comparison Matrix

| Feature | Keygen.sh | Lemon Squeezy | Paddle | Gumroad | Custom Server |
|---------|-----------|---------------|--------|---------|---------------|
| Dedicated licensing API | Yes | Basic | Basic | Minimal | Custom |
| Rust SDK | Unofficial | No | No | No | Native |
| Offline validation | Ed25519 signed | No | No | No | Custom |
| Device fingerprinting | Built-in | No | No | No | Custom |
| Seat management | Built-in | Activation limits | No | No | Custom |
| Key expiry | Built-in | No | No | No | Custom |
| Revocation | Built-in | Manual | Manual | Manual | Custom |
| Self-hosted option | CE (free) | No | No | No | Yes |
| Payment integration | Via Paddle/Stripe | Built-in (MoR) | Built-in (MoR) | Built-in | Separate |
| Monthly cost (low vol) | Free (100 ALU) | $0 + 5%/txn | $0 + 5%/txn | $0 + 10%/txn | Hosting only |
| Setup effort | Hours | Hours | Hours | Hours | Weeks |
| SOC 2 | Type II | Unknown | Unknown | No | N/A |

---

## 7. Sources

- Keygen.sh - Software Licensing API: https://keygen.sh
- Keygen Pricing: https://keygen.sh/pricing/
- Keygen Self-Hosting Docs: https://keygen.sh/docs/self-hosting/
- Keygen Offline Licensing Model: https://keygen.sh/docs/choosing-a-licensing-model/offline-licenses/
- Keygen Cryptographic API Reference: https://keygen.sh/docs/api/cryptography/
- keygen-rs Rust SDK: https://crates.io/crates/keygen-rs
- Lemon Squeezy License API: https://docs.lemonsqueezy.com/api/license-api
- Lemon Squeezy Pricing: https://www.lemonsqueezy.com/pricing
- Paddle Developer: https://developer.paddle.com/
- Paddle Pricing: https://www.paddle.com/pricing
- Gumroad License Keys: https://gumroad.com/help/article/76-license-keys
- device-fingerprint crate: https://crates.io/crates/device-fingerprint
- axum-jwt-auth: https://github.com/cmackenzie1/axum-jwt-auth
