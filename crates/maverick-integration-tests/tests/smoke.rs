use maverick_core::storage::InstallProfile;
use maverick_domain::RegionId;
use maverick_extension_contracts::{SyncBatchEnvelopeV1, SyncEventV1, EXTENSION_CONTRACT_VERSION};

#[test]
fn region_parse_roundtrip() {
    let r: RegionId = "EU868".parse().expect("parse");
    assert_eq!(r.to_string(), "EU868");
}

#[test]
fn install_profile_default_policy_serializes() {
    let p = InstallProfile::Balanced.default_storage_policy();
    let s = serde_json::to_string(&p).expect("json");
    assert!(s.contains("circular_at_hard_limit"));
}

#[tokio::test]
async fn sync_envelope_roundtrip_json() {
    let env = SyncBatchEnvelopeV1 {
        contract_version: EXTENSION_CONTRACT_VERSION.to_string(),
        edge_id: "edge-1".to_string(),
        batch_id: "b1".to_string(),
        created_at_ms: 1,
        events: vec![SyncEventV1 {
            correlation_id: "c1".to_string(),
            entity_type: "Device".to_string(),
            entity_id: Some("0102030405060708".to_string()),
            operation: "FrameReceived".to_string(),
            outcome: "Success".to_string(),
            metadata: None,
        }],
    };
    let j = serde_json::to_string(&env).unwrap();
    let back: SyncBatchEnvelopeV1 = serde_json::from_str(&j).unwrap();
    assert_eq!(back.batch_id, "b1");
}
