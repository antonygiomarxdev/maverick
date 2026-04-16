
const fs = require('fs');
const path = 'C:/Users/ksante/dev/maverick/.planning/research/SUMMARY.md';
const lines = [
  '# RESEARCH SUMMARY \u2014 Maverick LNS',
  '_Synthesized: 2026-04-16_',
  '',
  '---',
  '',
  '## TL;DR',
  '',
  '- **MIC verification and FCnt 32-bit reconstruction are not optional hardening \u2014 they are prerequisites for Maverick being a real LNS.** Any production ABP deployment before these land stores forgeable, session-breaking data. These two items must ship together in Phase 1.',
  '- **The UDP ingest surface is currently open by default (\u00600.0.0.0:17000\u0060) with no authentication and no MIC check.** Combined, these two gaps mean any LAN host can forge unlimited uplinks and fill the database. Changing the default bind to \u0060127.0.0.1\u0060 is a one-line fix that must accompany Phase 1.',
  '- **The recommended extension IPC is a local HTTP server (axum) with SSE push** \u2014 no Unix socket protocol, no gRPC, no message broker. \u0060axum\u0060 is already tokio-native; the only new dependency. Extensions reconnect with cursor-based catch-up; the core never blocks.',
  '- **SPI adapter is last, not first.** It depends on: correct protocol handling (Phase 1), the \u0060UplinkSource\u0060 port trait (Phase 2), and cannot be integration-tested without physical hardware. The safe v1 path is supervised \u0060lora_pkt_fwd\u0060 sibling process; FFI to \u0060libloragw\u0060 is the correct long-term target.',
  '- **The Rust async reliability gaps (Mutex poison from \u0060.expect()\u0060 in lns_ops.rs, \u0060std::thread::sleep\u0060 in \u0060spawn_blocking\u0060, \u0060process::exit\u0060 scattered across 25+ async paths) compound under failure conditions.** They must be addressed before process supervision is meaningful \u2014 a supervisor that restarts a permanently-bricked SQLite mutex is not useful.',
];
fs.writeFileSync(path, lines.join('\n'), 'utf8');
console.log('ok', lines.length, 'lines');
