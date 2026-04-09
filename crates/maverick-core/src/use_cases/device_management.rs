use maverick_domain::{AppKey, Device, DeviceClass, DeviceKeys, DeviceState, Eui64, NwkKey};

use crate::events::{AuditRecord, EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::ports::{AuditLogWriter, DeviceRepository};
use crate::{DomainError, Result};

#[derive(Debug, Clone)]
pub struct CreateDeviceCommand {
    pub dev_eui: Eui64,
    pub app_eui: Eui64,
    pub app_key: AppKey,
    pub nwk_key: NwkKey,
    pub class: DeviceClass,
}

#[derive(Debug, Clone)]
pub struct GetDeviceQuery {
    pub dev_eui: Eui64,
}

#[derive(Debug, Clone)]
pub struct UpdateDeviceCommand {
    pub dev_eui: Eui64,
    pub app_eui: Option<Eui64>,
    pub app_key: Option<AppKey>,
    pub nwk_key: Option<NwkKey>,
    pub class: Option<DeviceClass>,
    pub state: Option<DeviceState>,
}

#[derive(Debug, Clone)]
pub struct DeleteDeviceCommand {
    pub dev_eui: Eui64,
}

#[derive(Debug, Clone)]
pub struct OperationContext {
    pub source: EventSource,
    pub actor: String,
    pub correlation_id: Option<String>,
}

impl OperationContext {
    pub fn api(correlation_id: Option<String>) -> Self {
        Self {
            source: EventSource::Api,
            actor: "anonymous".to_string(),
            correlation_id,
        }
    }
}

pub struct DeviceManagementService<R, A> {
    repository: R,
    audit_log: A,
    event_bus: EventBus,
}

impl<R, A> DeviceManagementService<R, A>
where
    R: DeviceRepository,
    A: AuditLogWriter,
{
    pub fn new(repository: R, audit_log: A, event_bus: EventBus) -> Self {
        Self {
            repository,
            audit_log,
            event_bus,
        }
    }

    pub async fn create_device(
        &self,
        command: CreateDeviceCommand,
        context: OperationContext,
    ) -> Result<Device> {
        let dev_eui = command.dev_eui;
        let entity_id = dev_eui.to_string();

        if self.repository.get_by_dev_eui(dev_eui).await?.is_some() {
            let error = DomainError::AlreadyExists {
                entity: "device",
                id: entity_id.clone(),
            };
            self.record_failure("device.create", &entity_id, &context, &error)
                .await?;
            return Err(error.into());
        }

        let mut device = Device::new(
            dev_eui,
            command.app_eui,
            DeviceKeys::new(command.app_key, command.nwk_key),
        );
        device.class = command.class;

        let created = self.repository.create(device).await?;
        self.record_success(
            "device.create",
            &entity_id,
            "device",
            "device created",
            &context,
        )
        .await?;

        Ok(created)
    }

    pub async fn get_device(
        &self,
        query: GetDeviceQuery,
        context: OperationContext,
    ) -> Result<Device> {
        let entity_id = query.dev_eui.to_string();
        let device = self.repository.get_by_dev_eui(query.dev_eui).await?;

        match device {
            Some(device) => {
                self.record_success(
                    "device.get",
                    &entity_id,
                    "device",
                    "device fetched",
                    &context,
                )
                .await?;
                Ok(device)
            }
            None => {
                let error = DomainError::NotFound {
                    entity: "device",
                    id: entity_id.clone(),
                };
                self.record_failure("device.get", &entity_id, &context, &error)
                    .await?;
                Err(error.into())
            }
        }
    }

    pub async fn update_device(
        &self,
        command: UpdateDeviceCommand,
        context: OperationContext,
    ) -> Result<Device> {
        if command.app_eui.is_none()
            && command.app_key.is_none()
            && command.nwk_key.is_none()
            && command.class.is_none()
            && command.state.is_none()
        {
            let error = DomainError::Validation {
                field: "patch",
                reason: "at least one field must be provided".to_string(),
            };
            self.record_failure(
                "device.update",
                &command.dev_eui.to_string(),
                &context,
                &error,
            )
            .await?;
            return Err(error.into());
        }

        let entity_id = command.dev_eui.to_string();
        let mut device = match self.repository.get_by_dev_eui(command.dev_eui).await? {
            Some(device) => device,
            None => {
                let error = DomainError::NotFound {
                    entity: "device",
                    id: entity_id.clone(),
                };
                self.record_failure("device.update", &entity_id, &context, &error)
                    .await?;
                return Err(error.into());
            }
        };

        if let Some(app_eui) = command.app_eui {
            device.app_eui = app_eui;
        }
        if let Some(app_key) = command.app_key {
            device.keys.app_key = app_key;
        }
        if let Some(nwk_key) = command.nwk_key {
            device.keys.nwk_key = nwk_key;
        }
        if let Some(class) = command.class {
            device.class = class;
        }
        if let Some(state) = command.state {
            device.state = state;
        }

        let updated = self.repository.update(device).await?;
        self.record_success(
            "device.update",
            &entity_id,
            "device",
            "device updated",
            &context,
        )
        .await?;

        Ok(updated)
    }

    pub async fn delete_device(
        &self,
        command: DeleteDeviceCommand,
        context: OperationContext,
    ) -> Result<()> {
        let entity_id = command.dev_eui.to_string();

        if self
            .repository
            .get_by_dev_eui(command.dev_eui)
            .await?
            .is_none()
        {
            let error = DomainError::NotFound {
                entity: "device",
                id: entity_id.clone(),
            };
            self.record_failure("device.delete", &entity_id, &context, &error)
                .await?;
            return Err(error.into());
        }

        self.repository.delete(command.dev_eui).await?;
        self.record_success(
            "device.delete",
            &entity_id,
            "device",
            "device deleted",
            &context,
        )
        .await?;

        Ok(())
    }

    async fn record_success(
        &self,
        operation: &str,
        entity_id: &str,
        entity_type: &str,
        summary: &str,
        context: &OperationContext,
    ) -> Result<()> {
        let audit = self.audit_record(
            operation,
            entity_type,
            entity_id,
            EventStatus::Succeeded,
            summary,
            None,
            context,
        );
        self.audit_log.record(audit).await?;

        self.event_bus.publish(
            SystemEvent::new(
                EventKind::DeviceCommand,
                context.source.clone(),
                operation,
                EventStatus::Succeeded,
            )
            .with_entity_id(entity_id.to_string())
            .with_metadata("actor", context.actor.clone())
            .with_metadata("entity_type", entity_type.to_string())
            .with_metadata("summary", summary.to_string())
            .with_metadata("outcome", "succeeded")
            .with_correlation_id_option(context.correlation_id.clone()),
        );

        Ok(())
    }

    async fn record_failure(
        &self,
        operation: &str,
        entity_id: &str,
        context: &OperationContext,
        error: &DomainError,
    ) -> Result<()> {
        let reason_code = reason_code(error);
        let summary = error.to_string();
        let audit = self.audit_record(
            operation,
            "device",
            entity_id,
            EventStatus::Rejected,
            &summary,
            Some(reason_code.clone()),
            context,
        );
        self.audit_log.record(audit).await?;

        let mut event = SystemEvent::new(
            EventKind::DeviceCommand,
            context.source.clone(),
            operation,
            EventStatus::Rejected,
        )
        .with_entity_id(entity_id.to_string())
        .with_reason_code(reason_code)
        .with_metadata("actor", context.actor.clone())
        .with_metadata("summary", summary);

        if let Some(correlation_id) = &context.correlation_id {
            event = event.with_correlation_id(correlation_id.clone());
        }

        self.event_bus.publish(event);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn audit_record(
        &self,
        operation: &str,
        entity_type: &str,
        entity_id: &str,
        outcome: EventStatus,
        summary: &str,
        reason_code: Option<String>,
        context: &OperationContext,
    ) -> AuditRecord {
        let mut record = AuditRecord::new(
            context.source.clone(),
            operation,
            entity_type,
            outcome,
            summary,
        );
        record.entity_id = Some(entity_id.to_string());
        record.reason_code = reason_code;
        record.correlation_id = context.correlation_id.clone();
        record.with_metadata("actor", context.actor.clone())
    }
}

fn reason_code(error: &DomainError) -> String {
    match error {
        DomainError::NotFound { .. } => "device_not_found".to_string(),
        DomainError::AlreadyExists { .. } => "device_already_exists".to_string(),
        DomainError::Validation { .. } => "validation_failed".to_string(),
        DomainError::InvalidState { .. } => "invalid_state".to_string(),
    }
}

trait EventBuilderExt {
    fn with_correlation_id_option(self, correlation_id: Option<String>) -> Self;
}

impl EventBuilderExt for SystemEvent {
    fn with_correlation_id_option(mut self, correlation_id: Option<String>) -> Self {
        self.correlation_id = correlation_id;
        self
    }
}
