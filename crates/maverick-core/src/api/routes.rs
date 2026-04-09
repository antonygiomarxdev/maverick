use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{Json, Router};

use super::AppState;
use crate::api::dto::{
    parse_path_dev_eui, CreateDeviceRequestDto, CreateDownlinkRequestDto, DeviceResponseDto,
    DownlinkEnqueueResponseDto, DownlinkListQueryDto, DownlinkResponseDto, GatewayListQueryDto,
    GatewayResponseDto, PatchDeviceRequestDto,
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
        .route("/gateways", axum::routing::get(list_gateways::<D>))
        .route(
            "/gateways/healthy",
            axum::routing::get(list_healthy_gateways::<D>),
        )
        .route("/devices", axum::routing::post(create_device::<D>))
        .route(
            "/devices/:dev_eui/downlinks",
            axum::routing::get(list_downlinks::<D>).post(enqueue_downlink::<D>),
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

async fn list_gateways<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    Query(query): Query<GatewayListQueryDto>,
) -> crate::Result<Json<Vec<GatewayResponseDto>>> {
    let gateways =
        crate::ports::GatewayRepository::list(&state.services.gateway_repo, query.status_filter()?)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();

    Ok(Json(gateways))
}

async fn list_healthy_gateways<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
) -> crate::Result<Json<Vec<GatewayResponseDto>>> {
    let gateways = crate::ports::GatewayRepository::list_healthy(&state.services.gateway_repo)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Json(gateways))
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
    let dev_eui = parse_path_dev_eui(&dev_eui)?;
    let gateway_override = payload
        .gateway_eui
        .as_deref()
        .map(crate::api::dto::parse_path_dev_eui)
        .transpose()?;
    let draft = payload.into_draft(dev_eui)?;
    let outcome = service
        .enqueue(EnqueueDownlinkCommand {
            draft,
            gateway_override,
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

async fn list_downlinks<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    Path(dev_eui): Path<String>,
    Query(query): Query<DownlinkListQueryDto>,
) -> crate::Result<Json<Vec<DownlinkResponseDto>>> {
    let items = crate::ports::DownlinkRepository::list_by_dev_eui(
        &state.services.downlink_repo,
        parse_path_dev_eui(&dev_eui)?,
        query.state_filter()?,
        query.limit_or_default(),
    )
    .await?
    .into_iter()
    .map(Into::into)
    .collect();

    Ok(Json(items))
}

async fn get_downlink<D: Database + Clone + Send + Sync + 'static>(
    State(state): State<AppState<D>>,
    Path((dev_eui, downlink_id)): Path<(String, i64)>,
) -> crate::Result<Json<DownlinkResponseDto>> {
    let Some(downlink) =
        crate::ports::DownlinkRepository::get_by_id(&state.services.downlink_repo, downlink_id)
            .await?
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
) -> DeviceManagementService<
    &crate::adapters::persistence::SqliteDeviceRepository<D>,
    &crate::adapters::persistence::SqliteAuditLogWriter<D>,
> {
    state.services.device_service()
}

fn downlink_service<D: Database + Clone + Send + Sync + 'static>(
    state: &AppState<D>,
) -> ProcessDownlinkFrameService<
    &crate::adapters::persistence::SqliteDownlinkRepository<D>,
    &crate::adapters::persistence::SqliteGatewayRepository<D>,
    &crate::adapters::persistence::SqliteAuditLogWriter<D>,
> {
    state.services.downlink_service()
}

fn correlation_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}
