## Maverick Engineering Rules

- Keep the kernel pure: domain, ports, use cases, protocol policy, resilience and orchestration contracts belong in core; HTTP, UDP, persistence wiring, metrics, CLI and deployment concerns do not.
- Depend on traits, not concrete adapters. Composition of SQLite, network transports or framework-specific objects must happen outside kernel modules.
- Prefer small, explicit units with one reason to change. If a module is mixing policy and infrastructure, split it before adding more behavior.
- Preserve offline-first behavior. New flows must keep local processing and degradation paths working when remote dependencies fail.
- Make failure modes explicit. Return structured errors, avoid hidden panics, and add backpressure, retries or circuit breakers only where they protect a real boundary.
- Use Rust idioms that improve clarity: strong types, exhaustive matches, ownership-aware APIs, and async boundaries only where needed.
- Keep public APIs narrow. Expose the minimum surface required by callers and avoid leaking adapter details through core interfaces.
- Write tests close to the behavior being protected. Prefer focused unit tests for policy and targeted integration tests for adapters and composition.
- Do not move fast by violating boundaries. If a change needs a shortcut across layers, stop and introduce the missing port or composition seam first.