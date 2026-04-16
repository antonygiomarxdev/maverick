---
phase: 01-protocol-correctness
plan: F
type: execute
wave: 2
depends_on:
  - 01-A
files_modified:
  - crates/maverick-runtime-edge/src/cli_constants.rs
autonomous: true
requirements:
  - SEC-01
  - CORE-01

must_haves:
  truths:
    - "DEFAULT_GWMP_BIND_ADDR is '127.0.0.1:17000' (not 0.0.0.0)"
    - "No HTTP client crates (reqwest, hyper, h2, ureq) appear in maverick-runtime-edge dependency tree"
    - "No DNS lookup or external network calls in any source file under crates/maverick-runtime-edge/src/ or crates/maverick-core/src/"
  artifacts:
    - path: "crates/maverick-runtime-edge/src/cli_constants.rs"
      provides: "Updated DEFAULT_GWMP_BIND_ADDR constant"
      contains: "127.0.0.1:17000"
  key_links:
    - from: "crates/maverick-runtime-edge/src/main.rs"
      to: "crates/maverick-runtime-edge/src/cli_constants.rs"
      via: "DEFAULT_GWMP_BIND_ADDR used as default_value in IngestOnce and IngestLoop args"
      pattern: "DEFAULT_GWMP_BIND_ADDR"
---

<objective>
Change the UDP bind default from 0.0.0.0:17000 to 127.0.0.1:17000 and verify CORE-01 (zero external HTTP/DNS calls) by static analysis.

Purpose: SEC-01 reduces the attack surface — the GWMP socket is not accidentally exposed to external networks when running on a gateway with a public interface. CORE-01 is a correctness invariant (the LNS must work offline) that needs an explicit verification pass before Phase 1 ships.

Output: One-line constant change. Documentation of the CORE-01 audit outcome.
</objective>

<execution_context>
@/root/.claude/get-shit-done/workflows/execute-plan.md
@/root/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/01-protocol-correctness/1-CONTEXT.md
@.planning/phases/01-protocol-correctness/01-RESEARCH.md
</context>

<tasks>

<task type="auto">
  <name>Task F-1: Change DEFAULT_GWMP_BIND_ADDR to 127.0.0.1</name>
  <files>
    crates/maverick-runtime-edge/src/cli_constants.rs
  </files>
  <read_first>
    - crates/maverick-runtime-edge/src/cli_constants.rs
    - crates/maverick-runtime-edge/src/main.rs
  </read_first>
  <action>
In `cli_constants.rs`, change the value of `DEFAULT_GWMP_BIND_ADDR`:

```rust
// Before:
pub const DEFAULT_GWMP_BIND_ADDR: &str = "0.0.0.0:17000";

// After (per D-21 locked decision):
/// Default bind address for GWMP uplink ingest.
///
/// Changed from 0.0.0.0 to 127.0.0.1 (SEC-01) — binds to loopback only.
/// For external packet forwarders on other hosts, override with:
///   --bind 0.0.0.0:17000
/// or set MAVERICK_GWMP_BIND=0.0.0.0:17000 in the environment.
pub const DEFAULT_GWMP_BIND_ADDR: &str = "127.0.0.1:17000";
```

Also update the doc comment on `DEFAULT_RADIO_PROBE_HOST` if needed to stay consistent (it is already `"127.0.0.1"` so no change needed there).

No other files need modification — `DEFAULT_GWMP_BIND_ADDR` is used via `default_value = DEFAULT_GWMP_BIND_ADDR` in the `IngestOnce.bind` and `IngestLoop.bind` CLI args in `main.rs`, and via `gwmp_bind_effective()` in `commands.rs`. All callers pick up the new value automatically.

NOTE: `DEFAULT_RADIO_PROBE_PORT` (17000) is the probe port and is separate from the bind addr — do not change that constant.
  </action>
  <verify>
    <automated>grep -n "DEFAULT_GWMP_BIND_ADDR" crates/maverick-runtime-edge/src/cli_constants.rs</automated>
  </verify>
  <done>
    - `DEFAULT_GWMP_BIND_ADDR = "127.0.0.1:17000"` — grep confirms the value
    - `cargo check -p maverick-runtime-edge` passes (constant type is still `&str`)
  </done>
</task>

<task type="auto" tdd="false">
  <name>Task F-2: CORE-01 audit — verify zero external HTTP/DNS calls</name>
  <files>
    crates/maverick-runtime-edge/src/cli_constants.rs
  </files>
  <read_first>
    - crates/maverick-runtime-edge/Cargo.toml
    - crates/maverick-core/Cargo.toml
    - crates/maverick-adapter-persistence-sqlite/Cargo.toml
    - crates/maverick-adapter-radio-udp/Cargo.toml
    - Cargo.toml
  </read_first>
  <action>
This task is a STATIC ANALYSIS AUDIT. No code is written. The executor must run the following verification commands and record the results. If any finding is non-compliant, the executor must fix it before proceeding.

**Step 1: Check for HTTP client crates in the dependency tree**
```bash
cargo tree -p maverick-runtime-edge 2>&1 | grep -iE "reqwest|hyper|h2|ureq|isahc|attohttpc|curl"
```
Expected output: empty (no matches). These crates indicate HTTP client capability.

**Step 2: Check for DNS resolution in source code**
```bash
grep -rn "TcpStream::connect\|lookup_host\|resolve\|dns\|ToSocketAddrs\|reqwest\|hyper\|ureq" \
  crates/maverick-runtime-edge/src/ \
  crates/maverick-core/src/ \
  crates/maverick-adapter-persistence-sqlite/src/ \
  crates/maverick-adapter-radio-udp/src/ \
  2>/dev/null | grep -v "//.*\|test\|#\[" | head -20
```
Expected: No HTTP/DNS calls. UDP socket binds are acceptable (that is how GWMP ingest works).

**Step 3: Check for `reqwest`, `hyper`, or similar in all Cargo.toml files**
```bash
grep -rn "reqwest\|hyper\|ureq\|h2\|isahc" Cargo.toml crates/*/Cargo.toml 2>/dev/null
```
Expected: empty.

**Step 4: Verify UDP-only networking in maverick-adapter-radio-udp**
```bash
grep -n "UdpSocket\|TcpStream\|TcpListener" crates/maverick-adapter-radio-udp/src/*.rs
```
Expected: Only `UdpSocket` — no `TcpStream` or `TcpListener`.

**Step 5: Verify SQLite is bundled (no network SQLite)**
```bash
grep -n "bundled" crates/maverick-adapter-persistence-sqlite/Cargo.toml
```
Expected: `features = ["bundled"]` confirms SQLite is statically compiled in.

Record findings in the SUMMARY.md. If any check fails:
- Remove the offending dependency from `Cargo.toml`
- Replace TCP calls with `anyhow::bail!("not supported in offline mode")`
- Re-run the check

Add a doc comment to `cli_constants.rs` documenting the CORE-01 audit result:
```rust
// CORE-01 audit (Phase 1, 2026-04-16):
// maverick-runtime-edge dependency tree contains no HTTP client crates (reqwest/hyper/h2/ureq).
// Networking is limited to: UdpSocket (GWMP ingest), SQLite WAL (local file).
// No external DNS lookups or TCP connections in any core/adapter source file.
```
  </action>
  <verify>
    <automated>cargo tree -p maverick-runtime-edge 2>&1 | grep -iE "reqwest|hyper|h2|ureq|isahc" | wc -l</automated>
  </verify>
  <done>
    - Audit commands run and output recorded
    - Zero HTTP client crates in `maverick-runtime-edge` dependency tree
    - Zero DNS/TCP calls in core/adapter source files
    - CORE-01 audit doc comment added to `cli_constants.rs`
    - If any violation found: it is fixed and re-verified before this task is marked done
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| UDP socket bind address | Default determines what interfaces accept GWMP packets; 127.0.0.1 restricts to local only |
| External network calls | Any outbound TCP/HTTP from the LNS process breaks the offline-first invariant |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-F-01 | Elevation of Privilege | GWMP socket exposed on all interfaces | mitigate | DEFAULT_GWMP_BIND_ADDR changed to 127.0.0.1:17000; operator must explicitly opt-in to 0.0.0.0 for external packet forwarders |
| T-01-F-02 | Information Disclosure | External HTTP/DNS calls leaking device data | mitigate | CORE-01 static audit confirms no HTTP client crates in dependency tree; verified by cargo tree in Task F-2 |
| T-01-F-03 | Denial of Service | Operator accidentally breaks LNS by using external forwarder with default bind | accept | Doc comment on DEFAULT_GWMP_BIND_ADDR explains the opt-in; documented in runbook |
</threat_model>

<verification>
After both tasks complete:

```bash
grep "DEFAULT_GWMP_BIND_ADDR" crates/maverick-runtime-edge/src/cli_constants.rs
# Expected: "127.0.0.1:17000"

cargo tree -p maverick-runtime-edge 2>&1 | grep -iE "reqwest|hyper|h2" | wc -l
# Expected: 0

cargo check -p maverick-runtime-edge
# Expected: no errors
```
</verification>

<success_criteria>
- `DEFAULT_GWMP_BIND_ADDR = "127.0.0.1:17000"` — grep-verifiable
- `cargo tree -p maverick-runtime-edge | grep -iE "reqwest|hyper|h2"` returns empty — verifiable
- No `TcpStream` or DNS calls in core/adapter source files — grep-verifiable
- `cargo check -p maverick-runtime-edge` passes
</success_criteria>

<output>
After completion, create `.planning/phases/01-protocol-correctness/01-F-SUMMARY.md`
</output>
