use async_trait::async_trait;
use maverick_domain::{DevAddr, SessionSnapshot};

use crate::error::AppResult;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn get_by_dev_addr(&self, dev_addr: DevAddr) -> AppResult<Option<SessionSnapshot>>;
    async fn upsert(&self, session: &SessionSnapshot) -> AppResult<()>;
}
