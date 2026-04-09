# Business Rules & Domain Constraints

Este documento describe las reglas de negocio implementadas en Maverick que aseguran la integridad del dominio.

## Gateway Management

### Regla: Gateway EUI Uniqueness
- **Descripción**: No se pueden tener dos gateways con el mismo `gateway_eui`
- **Implementación**: `gateway_eui` es PRIMARY KEY en tabla `gateways`
- **Comportamiento**: 
  - INSERT duplicado → SQLite rechaza con constraint violation
   - UPDATE sobre gateway inexistente → dominio responde `NotFound`
- **Operaciones vigentes**: `create()` crea, `update()` modifica, `delete()` elimina y `list()`/`list_healthy()` exponen superficie operativa

### Regla: Gateway Status Validation
- **Valores válidos**: `Online`, `Offline`, `Timeout`
- **Implementación**: Enum `GatewayStatus` en dominio
- **Comportamiento**: Compilación falla si se usa estado inválido

### Regla: Automatic Gateway Selection for Downlinks
- **Descripción**: Si un downlink no especifica `gateway_eui`, Maverick debe seleccionar un gateway saludable automáticamente
- **Implementación**: `GatewaySelector` del kernel sobre `list_healthy()` del repositorio
- **Criterio actual**: score determinista por estado saludable, recencia (`last_seen`) y disponibilidad de telemetría básica
- **Comportamiento**: si no hay gateways saludables disponibles, el enqueue falla con conflicto de estado

## Device Management

### Regla: Device EUI Uniqueness
- **Descripción**: No se pueden tener dos dispositivos con el mismo `dev_eui`
- **Implementación**: `dev_eui` es PRIMARY KEY en tabla `devices`
- **Comportamiento**: Mismo que gateways

### Regla: App Key Requirement
- **Descripción**: Todo dispositivo requiere un `app_key` válido
- **Implementación**: NOT NULL constraint en `app_key`
- **Operación**: `DeviceManagementService` valida antes de persistir

### Regla: Network Key Requirement
- **Descripción**: Todo dispositivo requiere un `nwk_key` válido
- **Implementación**: NOT NULL constraint en `nwk_key`
- **Operación**: `DeviceManagementService` valida antes de persistir

## Session Management

### Regla: Device Address Uniqueness per Session
- **Descripción**: No se pueden tener dos sesiones con el mismo `dev_addr`
- **Implementación**: UNIQUE constraint en `dev_addr` en tabla `device_sessions`
- **Nota**: Esta es la dirección de red asignada tras JOIN

### Regla: One Session per Device
- **Descripción**: Solo puede haber una sesión activa por `dev_eui`
- **Implementación**: `dev_eui` es PRIMARY KEY en tabla `device_sessions`
- **Comportamiento**: UPDATE (reemplaza sesión anterior)

## Frame Processing

### Regla: Spreading Factor Validation
- **Valores válidos**: 7 - 12 (LoRaWAN standard)
- **Implementación**: `SpreadingFactor::new()` valida en construcción
- **Comportamiento**: `Option<Self>` - retorna None si SF es inválido

### Regla: Frequency Band Validation
- **Descripción**: Las frecuencias deben estar en el rango válido regional
- **Implementación**: Pendiente implementar con regional bands
- **Nota**: Actualmente acepta cualquier u32

## Data Retention

### Regla: Uplink Expiration
- **Descripción**: Los uplinks obsoletos se purgan automáticamente
- **Retención**: Configurable por profile de almacenamiento
- **Implementación**: Batch cleanup job based on `expires_at`

### Regla: Audit Log Expiration
- **Descripción**: Logs de auditoría se purgan tras período de retención
- **Retención**: Configurable por profile
- **Implementación**: Batch cleanup job based on `expires_at`

## Audit & Compliance

### Regla: Operation Tracking
- **Descripción**: Todo cambio importante se registra en audit log
- **Campos capturados**: source, operation, entity_type, entity_id, outcome, correlation_id
- **Implementación**: `AuditLogService` intercepta eventos de dominio

### Regla: Immutable Audit Records
- **Descripción**: Una vez creado, un registro de audit no puede modificarse
- **Implementación**: Solo INSERT permitido en `audit_log` (no UPDATE/DELETE)

---

## Implementación de Nuevas Reglas

Para agregar una nueva regla de negocio:

1. **Define en el modelo de dominio** (`crates/maverick-domain`)
   - Usa tipos fuertemente tipados (newtypes, enums)
   - Implementa validación en constructores

2. **Aplica en la base de datos** (`crates/maverick-core/src/db/schema.sql`)
   - Agrega constraints (PRIMARY KEY, UNIQUE, NOT NULL, CHECK)  
   - Crea índices si es necesario

3. **Implementa en el repositorio** (`crates/maverick-core/src/adapters/persistence/sqlite_*.rs`)
   - Convierte errores DB en dominio errors
   - Propaga violaciones como `AppError::DomainConstraintViolation`

4. **Documenta aquí** con descripción y comportamiento esperado

