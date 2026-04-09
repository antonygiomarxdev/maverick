use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{Json, Router};

use super::AppState;
use crate::adapters::persistence::{
    SqliteAuditLogWriter, SqliteDeviceRepository, SqliteDownlinkRepository,
};
use crate::api::dto::{
    parse_path_dev_eui, CreateDeviceRequestDto, CreateDownlinkRequestDto, DeviceResponseDto,
    DownlinkEnqueueResponseDto, DownlinkResponseDto, PatchDeviceRequestDto,
};
use crate::db::Database;
use crate::use_cases::{
    DeleteDeviceCommand, DeviceManagementService, EnqueueDownlinkCommand, GetDeviceQuery,
    OperationContext, ProcessDownlinkFrameService,
};
use crate::DomainError;

fn router<D: Database + Clone + Send + Sync + 'static>() -> Router<AppState<D>> {
    Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/devices", axum::routing::post(create_device::<D>))
        .route(
            "/devices/:dev_eui/downlinks",
            axum::routing::post(enqueue_downlink::<D>),
        )
        .route(
            "/devices/:dev_eui/downlinks/:downlink_id",
            axum::routing::get(get_downlink::<D>),
        )
        .route(
            "/devices/:dev_eui",
            axum::routing::get(get_device::<D>)
                .patch(update_device::<D>)
                .delete(delete_device::<D>),
        )
}

pub fn routes<D: Database + Clone + Send + Sync + 'static>() -> Router<AppState<D>> {
    router()
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    database: &'static str,
}

async fn health_check<D: Database + Clone + Send + Sync + 'static>(
    state: axum::extract::State<AppState<D>>,
) -> (StatusCode, axum::Json<HealthResponse>) {
    let db_status = match state.services.db.execute("SELECT 1").await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    let response = HealthResponse {
        status: "ok",
        version: state.services.version,
        database: db_status,
    };

    (StatusCode::OK, axum::Json(response))
}

async fn create_device<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    headers: HeaderMap,
    Json(payload): Json<CreateDeviceRequestDto>,
) -> crate::Result<(StatusCode, Json<DeviceResponseDto>)> {
    let service = device_service(&state);
    let correlation_id = correlation_id(&headers);
    let command = payload.into_command()?;
    let device = service
        .create_device(command, OperationContext::api(correlation_id))
        .await?;

    Ok((StatusCode::CREATED, Json(device.into())))
}

async fn get_device<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    headers: HeaderMap,
    Path(dev_eui): Path<String>,
) -> crate::Result<Json<DeviceResponseDto>> {
    let service = device_service(&state);
    let device = service
        .get_device(
            GetDeviceQuery {
                dev_eui: parse_path_dev_eui(&dev_eui)?,
            },
            OperationContext::api(correlation_id(&headers)),
        )
        .await?;

    Ok(Json(device.into()))
}

async fn update_device<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    headers: HeaderMap,
    Path(dev_eui): Path<String>,
    Json(payload): Json<PatchDeviceRequestDto>,
) -> crate::Result<Json<DeviceResponseDto>> {
    let service = device_service(&state);
    let command = payload.into_command(parse_path_dev_eui(&dev_eui)?)?;
    let device = service
        .update_device(command, OperationContext::api(correlation_id(&headers)))
        .await?;

    Ok(Json(device.into()))
}

async fn delete_device<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    headers: HeaderMap,
    Path(dev_eui): Path<String>,
) -> crate::Result<StatusCode> {
    let service = device_service(&state);
    service
        .delete_device(
            DeleteDeviceCommand {
                dev_eui: parse_path_dev_eui(&dev_eui)?,
            },
            OperationContext::api(correlation_id(&headers)),
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn enqueue_downlink<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    headers: HeaderMap,
    Path(dev_eui): Path<String>,
    Json(payload): Json<CreateDownlinkRequestDto>,
) -> crate::Result<(StatusCode, Json<DownlinkEnqueueResponseDto>)> {
    let service = downlink_service(&state);
    let downlink = payload.into_domain(parse_path_dev_eui(&dev_eui)?)?;
    let outcome = service
        .enqueue(EnqueueDownlinkCommand {
            downlink,
            correlation_id: correlation_id(&headers),
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(DownlinkEnqueueResponseDto {
            downlink_id: outcome.downlink_id,
            status: "Queued",
        }),
    ))
}

async fn get_downlink<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    Path((dev_eui, downlink_id)): Path<(String, i64)>,
) -> crate::Result<Json<DownlinkResponseDto>> {
    let repository = SqliteDownlinkRepository::new(state.services.db.clone());
    let Some(downlink) =
        crate::ports::DownlinkRepository::get_by_id(&repository, downlink_id).await?
    else {
        return Err(DomainError::NotFound {
            entity: "downlink",
            id: downlink_id.to_string(),
        }
        .into());
    };

    let target_dev_eui = parse_path_dev_eui(&dev_eui)?;
    if downlink.downlink.dev_eui != target_dev_eui {
        return Err(DomainError::NotFound {
            entity: "downlink",
            id: downlink_id.to_string(),
        }
        .into());
    }

    Ok(Json(downlink.into()))
}

fn device_service<D: Database + Clone + Send + Sync + 'static>(
    state: &AppState<D>,
) -> DeviceManagementService<SqliteDeviceRepository<D>, SqliteAuditLogWriter<D>> {
    DeviceManagementService::new(
        SqliteDeviceRepository::new(state.services.db.clone()),
        SqliteAuditLogWriter::new(state.services.db.clone()),
        state.services.event_bus.clone(),
    )
}

fn downlink_service<D: Database + Clone + Send + Sync + 'static>(
    state: &AppState<D>,
) -> ProcessDownlinkFrameService<SqliteDownlinkRepository<D>, SqliteAuditLogWriter<D>> {
    ProcessDownlinkFrameService::new(
        SqliteDownlinkRepository::new(state.services.db.clone()),
        SqliteAuditLogWriter::new(state.services.db.clone()),
        state.services.event_bus.clone(),
    )
}

fn correlation_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}
