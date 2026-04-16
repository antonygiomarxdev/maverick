# Maverick — Vision

**Maverick es un LNS offline-first, self-contained, que trata la integridad de datos como innegociable: construido para correr donde nada más lo hace, y extensible por una comunidad que lo hace crecer.**

---

## Qué Es

Maverick es un servidor de red LoRaWAN (LNS) diseñado para despliegues en edge donde la conectividad es nula, intermitente o inestable.

**核心 valor:** Nunca perder un uplink — desde el radio hasta SQLite, los datos se preservan sin importar si hay internet, si las extensiones fallan, o si el proceso se reinicia.

---

## Lo Que Es y Lo Que No Es

### Es

- Un **stack completo** que se instala y funciona: LNS + radio directo (SX1302/3), sin dependencias externas
- **Offline-first**: cero llamados a la nube en el runtime core; toda persistencia es local
- **Extensible**: todo es opcional — TUI, dashboard, HTTP, MQTT, webhooks, AI — instalado y configurado por el operador
- **Aislado**: las extensiones son procesos separados que nunca afectan la estabilidad del LNS core
- **Opensource**: la comunidad contribuye al core, extensiones, documentación, compatibilidad de hardware
- **Compatible con AI**: extensiones oficiales que aprovechan APIs de AI (Claude, OpenAI); puerta abierta para ML local en hardware más capaz

### No Es

- Un servicio cloud
- Dependiente de conectividad
- Un producto cerrado o monolítico
- Diseñado para Windows o macOS (Linux only)
- Un reemplazo de TTN/The Things Stack (puede integrarse con ellos)

---

## Principios

### 1. Fiabilidad sobre todo

El LNS core nunca se cae, nunca pierde datos, nunca se bloquea por causas externas. Si el dashboard falla, el LNS sigue. Si el internet se va, el LNS sigue. Si una extensión tiene un bug, el LNS sigue.

### 2. Instalación trivial, configuración flexible

Con `maverick install` tenés un LNS funcional. Después, el operador elige qué extensiones instalar y cómo configurarlas via CLI interactiva o archivos de config.

### 3. El Edge es la fuente de verdad

El edge autoridad sobre sus datos. Maverick Cloud recibe sincronización, pero no es necesaria para el funcionamiento. Cuando la conectividad regresa, se sincroniza lo pendiente.

### 4. Extensiones como ciudadanos de primera clase

Las extensiones no son second-class citizens. Tienen la misma calidad y documentación que el core. La comunidad puede contribuir extensiones oficiales y community-driven.

### 5. AI desde el core, no en el core

El core expone datos de forma clara para que extensiones AI los consuman. No hay LLM corriendo en el edge por default (hardware limitado), pero la puerta está abierta para cuando el hardware lo permita.

---

## Arquitectura de Extensiones

```
┌─────────────────────────────────────────────────────────┐
│                     maverick-edge                       │
│                   (LNS Core - siempre arriba)            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────────┐  │
│  │ SQLite  │  │ Radio   │  │  CLI    │  │ Extension │  │
│  │ persist │  │ adapter │  │ management│ │   IPC     │  │
│  └─────────┘  └─────────┘  └─────────┘  └───────────┘  │
└─────────────────────────────────────────────────────────┘
                              │
                              │ Unix pipes / TCP / HTTP local
                              ▼
        ┌──────────────────┼───────────────────┬──────────┐
        ▼                  ▼                   ▼          ▼
  ┌──────────┐      ┌──────────┐      ┌──────────┐ ┌────────┐
  │ maverick │      │  HTTP    │      │  MQTT    │ │  AI    │
  │   TUI    │      │ outbound │      │ outbound │ │ext     │
  └──────────┘      └──────────┘      └──────────┘ └────────┘
        │                  │                   │         │
        ▼                  ▼                   ▼         ▼
     Operator           Cloud              Broker   OpenAI/
     local              sync              (local)   Claude
                                              │
                                              ▼
                                           MQTT
                                           broker
```

**Todas las extensiones son procesos separados.** Si cualquiera falla, el LNS sigue. El operador elige qué instalar.

---

## Sync con Maverick Cloud

**Modelo:** Muchos edges → un cloud (abierto a muchos→muchos).

- El edge **empuja** cuando tiene conectividad — no al revés
- Conexión puede ser intermitente, lenta, o nula por días
- Protocolo eficiente (MQTT o HTTPS con queue)
- El edge mantiene cola de eventos pendientes con timestamps
- **Eventual consistency**: cuando hay red, se sincroniza; cuando no hay, se acumula
- Auth: token por edge + TLS
- Conflictos: **cloud no gana** — el edge es source of truth para sus datos locales

### Datos sync

Por definir (configurable):
- Uplinks + sesiones (core)
- Métricas operativas
- Logs (opcional, no por defecto)

---

## Instalación y Setup

###Primera vez (CLI interactiva)

```bash
maverick install
# Región LoRaWAN
# Hardware de radio detectado
# Extensiones a instalar (ninguna por defecto)
# Credenciales / config inicial
```

Soporta deployment headless (SSH + config file o serial para setup inicial).

### Operación continua

- CLI: `maverick device add ...`, `maverick config set ...`
- Las extensiones se configuran via su propia CLI o archivo de config
- Todo es gestionable via SSH

### Updates

Por definir (OTA o manual).

---

## Retención de Datos

- **Core value**: nunca se pierde un uplink
- Por default: persists indefinitely en SQLite local
- Buffer circular configurable para proteger storage limitado (SD cards)
- Estrategia de cleanup: configurable, no destructivo por defecto

---

## Hardware

- **Target principal**: Raspberry Pi 3/4 (armv7, aarch64) con concentrador RAK LoRa (SX1302/3)
- **Mínimo**: armv7, 512 MB RAM, Linux
- **Escalable**: binarios para x86_64, aarch64, armv7
- **Extensible**: comunidad valida y extiende compatibilidad de hardware

---

## Comunidad y Opensource

Maverick es opensource y la contribución es bienvenida en todas las áreas:

- **Core**: bug fixes, features, protocol compliance
- **Extensiones**: oficiales y community-driven
- **Hardware**: drivers, compatibility testing
- **Documentación**: guías, tutorials, case studies
- **AI integrations**: nuevos providers, local ML

### Licencia

Por definir.

---

## Roadmap (Estado Actual)

| Fase | Descripción | Estado |
|------|-------------|--------|
| 01 | Protocol Correctness (MIC, FCnt 32-bit, FRMPayload) | ✅ Complete |
| 02 | Radio Abstraction & SPI | ✅ Complete |
| 03 | Class A Downlink | 🔲 Pendiente |
| 04 | ? | 🔲 |
| 05 | ? | 🔲 |

---

## Constraints técnicos

- **Rust**: hexagonal architecture, clean code
- **Offline-first**: cero cloud calls en core
- **Process isolation**: extensions son procesos separados
- **Linux only**
- **≤512 MB RAM** en target mínimo
- **Compatibility**: `lns-config.toml` no rompe entre versiones

---

_Last updated: 2026-04-16_
