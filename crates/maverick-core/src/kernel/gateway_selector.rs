use maverick_domain::{Eui64, Gateway};

use crate::ports::GatewayRepository;
use crate::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct GatewayCandidateScore {
    pub gateway_eui: Eui64,
    pub score: i64,
}

pub struct GatewaySelector<R> {
    repository: R,
}

impl<R> GatewaySelector<R>
where
    R: GatewayRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn healthy_candidates(&self) -> Result<Vec<GatewayCandidateScore>> {
        let now = unix_timestamp();
        let mut gateways: Vec<_> = self
            .repository
            .list_healthy()
            .await?
            .into_iter()
            .map(|gateway| GatewayCandidateScore {
                gateway_eui: gateway.gateway_eui,
                score: score_gateway(&gateway, now),
            })
            .collect();
        gateways.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.gateway_eui.as_bytes().cmp(&right.gateway_eui.as_bytes()))
        });
        Ok(gateways)
    }

    pub async fn select_best(&self) -> Result<Option<Gateway>> {
        let now = unix_timestamp();
        let mut gateways = self.repository.list_healthy().await?;
        gateways.sort_by(|left, right| {
            score_gateway(right, now)
                .cmp(&score_gateway(left, now))
                .then_with(|| left.gateway_eui.as_bytes().cmp(&right.gateway_eui.as_bytes()))
        });
        Ok(gateways.into_iter().next())
    }
}

fn score_gateway(gateway: &Gateway, now: i64) -> i64 {
    let recency_score = gateway
        .last_seen
        .map(|last_seen| (1_000 - (now - last_seen).max(0)).clamp(0, 1_000))
        .unwrap_or(0);
    let telemetry_score = i64::from(gateway.tx_frequency.is_some()) * 100
        + i64::from(gateway.rx_temperature.is_some()) * 25
        + i64::from(gateway.tx_temperature.is_some()) * 25;
    let thermal_penalty = gateway
        .tx_temperature
        .map(|temp| if temp >= 85.0 { 300 } else { 0 })
        .unwrap_or(0)
        + gateway
            .rx_temperature
            .map(|temp| if temp >= 85.0 { 300 } else { 0 })
            .unwrap_or(0);

    recency_score + telemetry_score - thermal_penalty
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use maverick_domain::{Eui64, Gateway, GatewayStatus};

    use super::GatewaySelector;
    use crate::ports::GatewayRepository;

    #[derive(Clone)]
    struct InMemoryGatewayRepository {
        gateways: Vec<Gateway>,
    }

    #[async_trait]
    impl GatewayRepository for InMemoryGatewayRepository {
        async fn create(&self, gateway: Gateway) -> crate::Result<Gateway> {
            Ok(gateway)
        }

        async fn update(&self, gateway: Gateway) -> crate::Result<Gateway> {
            Ok(gateway)
        }

        async fn delete(&self, _gateway_eui: Eui64) -> crate::Result<()> {
            Ok(())
        }

        async fn get_by_gateway_eui(&self, gateway_eui: Eui64) -> crate::Result<Option<Gateway>> {
            Ok(self
                .gateways
                .iter()
                .find(|gateway| gateway.gateway_eui == gateway_eui)
                .cloned())
        }

        async fn list(&self, status: Option<GatewayStatus>) -> crate::Result<Vec<Gateway>> {
            Ok(self
                .gateways
                .iter()
                .filter(|gateway| status.map(|value| gateway.status == value).unwrap_or(true))
                .cloned()
                .collect())
        }

        async fn list_healthy(&self) -> crate::Result<Vec<Gateway>> {
            self.list(Some(GatewayStatus::Online)).await
        }
    }

    #[tokio::test]
    async fn select_best_prefers_recent_gateway_with_telemetry() {
        let mut stale = Gateway::new(Eui64::from([1, 1, 1, 1, 1, 1, 1, 1]));
        stale.status = GatewayStatus::Online;
        stale.last_seen = Some(10);

        let mut recent = Gateway::new(Eui64::from([2, 2, 2, 2, 2, 2, 2, 2]));
        recent.status = GatewayStatus::Online;
        recent.last_seen = Some(super::unix_timestamp());
        recent.tx_frequency = Some(868_100_000);
        recent.rx_temperature = Some(35.0);

        let selector = GatewaySelector::new(InMemoryGatewayRepository {
            gateways: vec![stale, recent.clone()],
        });

        let selected = selector
            .select_best()
            .await
            .expect("selection must succeed")
            .expect("must return one gateway");

        assert_eq!(selected.gateway_eui, recent.gateway_eui);
    }

    #[tokio::test]
    async fn select_best_penalizes_overheated_gateway() {
        let now = super::unix_timestamp();

        let mut hot = Gateway::new(Eui64::from([3, 3, 3, 3, 3, 3, 3, 3]));
        hot.status = GatewayStatus::Online;
        hot.last_seen = Some(now);
        hot.tx_frequency = Some(868_100_000);
        hot.tx_temperature = Some(95.0);

        let mut cool = Gateway::new(Eui64::from([4, 4, 4, 4, 4, 4, 4, 4]));
        cool.status = GatewayStatus::Online;
        cool.last_seen = Some(now - 1);
        cool.tx_frequency = Some(868_100_000);
        cool.tx_temperature = Some(40.0);

        let selector = GatewaySelector::new(InMemoryGatewayRepository {
            gateways: vec![hot, cool.clone()],
        });

        let selected = selector
            .select_best()
            .await
            .expect("selection must succeed")
            .expect("must return one gateway");

        assert_eq!(selected.gateway_eui, cool.gateway_eui);
    }
}