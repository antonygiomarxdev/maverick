//! Guided LoRaWAN/LNS editing (applications, devices, autoprovision) with TOML + optional `config load`.

use std::path::Path;

use maverick_core::lns_config::{
    AbpKeys, ActivationMode, ApplicationEntry, DeviceEntry, LnsConfigDocument, OtaaKeys,
};
use std::io::Write;

use crate::config::TuiConfig;
use crate::console_ui::{pause_continue, prompt_with_default, prompt_yes_no};
use crate::edge_runner::run_edge_command_or_sudo;
use crate::lns_file::{
    load_or_default, save_lns_document, SaveLnsOutcome, LNS_CONFIG_DEFAULT_PATH,
};

fn path() -> &'static Path {
    Path::new(LNS_CONFIG_DEFAULT_PATH)
}

fn ensure_doc_available(cfg: &TuiConfig) -> Result<LnsConfigDocument, String> {
    let (mut doc, existed) = load_or_default(path())?;
    if existed {
        return Ok(doc);
    }
    println!("No file at {} yet.", LNS_CONFIG_DEFAULT_PATH);
    println!("Create a starter with: sudo maverick-edge config init --config-path {LNS_CONFIG_DEFAULT_PATH}");
    if prompt_yes_no(
        "Run `config init` now (uses sudo automatically if /etc/maverick is not writable)",
        true,
    )? {
        run_edge_command_or_sudo(
            cfg,
            &["config", "init", "--config-path", LNS_CONFIG_DEFAULT_PATH],
        )?;
        let (d, ex) = load_or_default(path())?;
        if !ex {
            return Err("config init reported success but file still missing".to_string());
        }
        doc = d;
        return Ok(doc);
    }
    Err("No lns-config.toml; cancelled.".to_string())
}

fn print_validation_err(e: &str) {
    println!("Validation error: {e}");
}

/// Indices of devices whose `application_id` equals `app_id`.
fn device_indices_with_application_id(doc: &LnsConfigDocument, app_id: &str) -> Vec<usize> {
    doc.devices
        .iter()
        .enumerate()
        .filter(|(_, d)| d.application_id == app_id)
        .map(|(i, _)| i)
        .collect()
}

const PREVIEW_DEVICE_ROWS_RENAME: usize = 8;
const PREVIEW_DEVICE_ROWS_REMOVE: usize = 5;

fn prompt_activation_mode() -> Result<ActivationMode, String> {
    println!("Activation mode:");
    println!("  1) OTAA (JoinEUI + AppKey; DevAddr optional until join / assign)");
    println!("  2) ABP (static DevAddr required)");
    let raw = prompt_with_default("Choose 1 or 2", "1")?;
    match raw.trim() {
        "1" => Ok(ActivationMode::Otaa),
        "2" => Ok(ActivationMode::Abp),
        _ => Err("invalid choice (use 1 or 2)".to_string()),
    }
}

fn prompt_application_id(doc: &LnsConfigDocument) -> Result<String, String> {
    println!("Applications:");
    for (i, a) in doc.applications.iter().enumerate() {
        println!("  [{}] {}", i + 1, a.id);
    }
    println!("  [m] Enter application id manually");
    let raw = prompt_with_default("Select number or m", "1")?;
    let t = raw.trim().to_ascii_lowercase();
    if t == "m" || t == "manual" {
        return prompt_with_default("application_id", "default");
    }
    let n: usize = t.parse().map_err(|_| "invalid number".to_string())?;
    if n == 0 || n > doc.applications.len() {
        return Err("application index out of range".to_string());
    }
    Ok(doc.applications[n - 1].id.clone())
}

fn prompt_otaa_keys() -> Result<OtaaKeys, String> {
    let join_eui = prompt_with_default("join_eui (16 hex)", "0000000000000000")?;
    let app_key = prompt_with_default("app_key (32 hex)", &"0".repeat(32))?;
    let nwk = prompt_with_default("nwk_key (32 hex, empty to omit)", "")?;
    Ok(OtaaKeys {
        join_eui,
        app_key,
        nwk_key: if nwk.trim().is_empty() {
            None
        } else {
            Some(nwk)
        },
    })
}

fn prompt_optional_abp_keys() -> Result<Option<AbpKeys>, String> {
    if !prompt_yes_no("Add optional ABP session keys (AppSKey / NwkSKey)", false)? {
        return Ok(None);
    }
    let apps = prompt_with_default("apps_key (32 hex, empty to skip)", "")?;
    let nwks = prompt_with_default("nwks_key (32 hex, empty to skip)", "")?;
    Ok(Some(AbpKeys {
        apps_key: if apps.trim().is_empty() {
            None
        } else {
            Some(apps)
        },
        nwks_key: if nwks.trim().is_empty() {
            None
        } else {
            Some(nwks)
        },
    }))
}

fn save_and_offer_load(cfg: &TuiConfig, doc: &LnsConfigDocument) -> Result<(), String> {
    match save_lns_document(path(), doc) {
        Ok(SaveLnsOutcome::Ok) => println!("Saved {}.", LNS_CONFIG_DEFAULT_PATH),
        Ok(SaveLnsOutcome::WroteTemp { temp_path, target }) => {
            println!(
                "Could not write {} (permission). Wrote:\n  {}",
                target.display(),
                temp_path.display()
            );
            println!("Then: sudo cp {} {}", temp_path.display(), target.display());
        }
        Err(e) => return Err(e),
    }
    if prompt_yes_no("Run `config load` to apply this file to SQLite now", true)? {
        run_edge_command_or_sudo(
            cfg,
            &["config", "load", "--config-path", LNS_CONFIG_DEFAULT_PATH],
        )?;
        println!("config load completed. Check: maverick-edge config show");
    }
    Ok(())
}

/// List / add / edit / remove applications.
pub(crate) fn run_applications_wizard(cfg: &TuiConfig) -> Result<(), String> {
    let mut doc = ensure_doc_available(cfg)?;
    loop {
        println!("\n--- Applications ({}) ---", doc.applications.len());
        for (i, a) in doc.applications.iter().enumerate() {
            println!(
                "  [{}] id={}  name={}  default_region={}",
                i + 1,
                a.id,
                a.name,
                a.default_region
            );
        }
        println!("  [a] Add   [e] Edit #   [r] Remove #   [s] Save + optional load   [b] Back");
        print!("Choice: ");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        let c = line.trim().to_ascii_lowercase();
        match c.as_str() {
            "b" | "q" => return Ok(()),
            "a" => {
                let id = prompt_with_default("Application id (unique)", "my-app")?;
                if doc.applications.iter().any(|x| x.id == id) {
                    println!("Duplicate id.");
                    continue;
                }
                let name = prompt_with_default("Display name", &id)?;
                let default_region = prompt_with_default("Default region (e.g. EU868)", "EU868")?;
                doc.applications.push(ApplicationEntry {
                    id,
                    name,
                    default_region,
                });
                if let Err(e) = doc.validate() {
                    print_validation_err(&e);
                    doc.applications.pop();
                }
            }
            "e" => {
                let idx_s = prompt_with_default("Edit index (1-based)", "1")?;
                let idx: usize = idx_s.parse().map_err(|_| "invalid index".to_string())?;
                if idx == 0 || idx > doc.applications.len() {
                    println!("Out of range.");
                    continue;
                }
                let i = idx - 1;
                let orig = doc.applications[i].clone();
                let orig_app_id_for_devices = orig.id.clone();
                let mut draft = orig.clone();
                draft.id = prompt_with_default("Application id", &draft.id)?;
                draft.name = prompt_with_default("Display name", &draft.name)?;
                draft.default_region =
                    prompt_with_default("Default region", &draft.default_region)?;

                let old_id = orig.id.as_str();
                let new_id = draft.id.as_str();
                let mut migrated_device_indices: Vec<usize> = Vec::new();
                if old_id != new_id {
                    let affected = device_indices_with_application_id(&doc, old_id);
                    if !affected.is_empty() {
                        println!("Renaming application id will update device rows:");
                        println!("  {old_id}  ->  {new_id}");
                        println!("Affected devices: {}", affected.len());
                        for di in affected.iter().take(PREVIEW_DEVICE_ROWS_RENAME) {
                            println!("  - dev_eui={}", doc.devices[*di].dev_eui);
                        }
                        if affected.len() > PREVIEW_DEVICE_ROWS_RENAME {
                            println!(
                                "  ... and {} more.",
                                affected.len() - PREVIEW_DEVICE_ROWS_RENAME
                            );
                        }
                        if !prompt_yes_no("Apply this rename and update those devices?", true)? {
                            println!("Cancelled.");
                            continue;
                        }
                    }
                    for di in &affected {
                        doc.devices[*di].application_id = draft.id.clone();
                    }
                    migrated_device_indices = affected;
                }

                doc.applications[i] = draft.clone();
                match doc.validate() {
                    Ok(()) => println!("OK (in memory; use [s] to save)."),
                    Err(e) => {
                        print_validation_err(&e);
                        doc.applications[i] = orig;
                        for di in migrated_device_indices {
                            if let Some(d) = doc.devices.get_mut(di) {
                                d.application_id = orig_app_id_for_devices.clone();
                            }
                        }
                    }
                }
            }
            "r" => {
                let idx_s = prompt_with_default("Remove index (1-based)", "1")?;
                let idx: usize = idx_s.parse().map_err(|_| "invalid index".to_string())?;
                if idx == 0 || idx > doc.applications.len() {
                    println!("Out of range.");
                    continue;
                }
                let app_index = idx - 1;
                let app_id = doc.applications[app_index].id.clone();
                let refs = device_indices_with_application_id(&doc, app_id.as_str());
                if !refs.is_empty() {
                    println!(
                        "Cannot remove application {:?}: {} device(s) still reference it.",
                        app_id,
                        refs.len()
                    );
                    for di in refs.iter().take(PREVIEW_DEVICE_ROWS_REMOVE) {
                        let d = &doc.devices[*di];
                        println!(
                            "  - dev_eui={} application_id={}",
                            d.dev_eui, d.application_id
                        );
                    }
                    if refs.len() > PREVIEW_DEVICE_ROWS_REMOVE {
                        println!(
                            "  ... and {} more.",
                            refs.len() - PREVIEW_DEVICE_ROWS_REMOVE
                        );
                    }
                    println!(
                        "Reassign or remove those devices in the Devices wizard first, then try again."
                    );
                    continue;
                }
                if !prompt_yes_no("Remove this application?", false)? {
                    continue;
                }
                doc.applications.remove(app_index);
                if let Err(e) = doc.validate() {
                    print_validation_err(&e);
                }
            }
            "s" => {
                if let Err(e) = doc.validate() {
                    print_validation_err(&e);
                    continue;
                }
                save_and_offer_load(cfg, &doc)?;
            }
            "" => {}
            _ => println!("Unknown choice."),
        }
    }
}

/// List / add / edit / remove devices.
pub(crate) fn run_devices_wizard(cfg: &TuiConfig) -> Result<(), String> {
    let mut doc = ensure_doc_available(cfg)?;
    loop {
        println!("\n--- Devices ({}) ---", doc.devices.len());
        if doc.applications.is_empty() {
            println!("Add at least one application first (Applications wizard).");
            pause_continue()?;
            return Ok(());
        }
        for (i, d) in doc.devices.iter().enumerate() {
            let addr = d
                .dev_addr
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("(none)");
            println!(
                "  [{}] mode={:?} dev_eui={} dev_addr={} app={} region={} enabled={}",
                i + 1,
                d.activation_mode,
                d.dev_eui,
                addr,
                d.application_id,
                d.region,
                d.enabled
            );
        }
        println!("  [a] Add   [e] Edit #   [r] Remove #   [s] Save + optional load   [b] Back");
        println!("  Tip: edits apply in memory; use [s] to write TOML and optionally load SQLite.");
        print!("Choice: ");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        match line.trim().to_ascii_lowercase().as_str() {
            "b" | "q" => return Ok(()),
            "a" => {
                let activation_mode = prompt_activation_mode()?;
                let application_id = loop {
                    let id = prompt_application_id(&doc)?;
                    if doc.applications.iter().any(|a| a.id == id) {
                        break id;
                    }
                    println!("Unknown application_id.");
                };
                let dev_eui = prompt_with_default("dev_eui (16 hex)", "0102030405060708")?;
                let region = prompt_with_default("region", "EU868")?;
                let en = prompt_with_default("enabled (true/false)", "true")?;
                let enabled = matches!(en.to_ascii_lowercase().as_str(), "true" | "1" | "yes");
                let (dev_addr, otaa, abp) = match activation_mode {
                    ActivationMode::Otaa => {
                        let otaa = Some(prompt_otaa_keys()?);
                        let da = if prompt_yes_no(
                            "Set optional static dev_addr now (8 hex; skip if unknown)",
                            false,
                        )? {
                            let s = prompt_with_default("dev_addr (8 hex)", "")?;
                            if s.trim().is_empty() {
                                None
                            } else {
                                Some(s)
                            }
                        } else {
                            None
                        };
                        (da, otaa, None)
                    }
                    ActivationMode::Abp => {
                        let da = Some(prompt_with_default(
                            "dev_addr (8 hex, required for ABP)",
                            "01ABCDEF",
                        )?);
                        let abp = prompt_optional_abp_keys()?;
                        (da, None, abp)
                    }
                };
                let entry = DeviceEntry {
                    activation_mode,
                    dev_eui,
                    dev_addr,
                    application_id,
                    region,
                    enabled,
                    otaa,
                    abp,
                };
                doc.devices.push(entry);
                if let Err(e) = doc.validate() {
                    print_validation_err(&e);
                    doc.devices.pop();
                }
            }
            "e" => {
                let idx_s = prompt_with_default("Edit index (1-based)", "1")?;
                let idx: usize = idx_s.parse().map_err(|_| "invalid index".to_string())?;
                if idx == 0 || idx > doc.devices.len() {
                    println!("Out of range.");
                    continue;
                }
                let orig = doc.devices[idx - 1].clone();
                let mut draft = orig.clone();
                let mode_raw = prompt_with_default(
                    "activation_mode (otaa or abp)",
                    match draft.activation_mode {
                        ActivationMode::Otaa => "otaa",
                        ActivationMode::Abp => "abp",
                    },
                )?;
                draft.activation_mode = match mode_raw.trim().to_ascii_lowercase().as_str() {
                    "otaa" => ActivationMode::Otaa,
                    "abp" => ActivationMode::Abp,
                    _ => {
                        println!("Invalid mode; use otaa or abp.");
                        continue;
                    }
                };
                draft.dev_eui = prompt_with_default("dev_eui", &draft.dev_eui)?;
                match draft.activation_mode {
                    ActivationMode::Otaa => {
                        let da = prompt_with_default(
                            "dev_addr (8 hex, empty if unknown)",
                            draft.dev_addr.as_deref().unwrap_or(""),
                        )?;
                        draft.dev_addr = if da.trim().is_empty() { None } else { Some(da) };
                        if draft.otaa.is_none()
                            || prompt_yes_no("Replace OTAA keys from prompts", false)?
                        {
                            draft.otaa = Some(prompt_otaa_keys()?);
                        }
                        draft.abp = None;
                    }
                    ActivationMode::Abp => {
                        draft.dev_addr = Some(prompt_with_default(
                            "dev_addr (8 hex)",
                            draft.dev_addr.as_deref().unwrap_or(""),
                        )?);
                        draft.otaa = None;
                        if prompt_yes_no("Replace optional ABP keys from prompts", false)? {
                            draft.abp = prompt_optional_abp_keys()?;
                        }
                    }
                }
                draft.application_id = loop {
                    let id = prompt_application_id(&doc)?;
                    if doc.applications.iter().any(|a| a.id == id) {
                        break id;
                    }
                    println!("Unknown application_id.");
                };
                draft.region = prompt_with_default("region", &draft.region)?;
                let en = prompt_with_default("enabled (true/false)", &draft.enabled.to_string())?;
                draft.enabled = matches!(en.to_ascii_lowercase().as_str(), "true" | "1" | "yes");
                let i = idx - 1;
                doc.devices[i] = draft.clone();
                match doc.validate() {
                    Ok(()) => println!("OK (in memory; use [s] to save file + optional load)."),
                    Err(e) => {
                        print_validation_err(&e);
                        doc.devices[i] = orig;
                    }
                }
            }
            "r" => {
                let idx_s = prompt_with_default("Remove index (1-based)", "1")?;
                let idx: usize = idx_s.parse().map_err(|_| "invalid index".to_string())?;
                if idx == 0 || idx > doc.devices.len() {
                    println!("Out of range.");
                    continue;
                }
                if prompt_yes_no("Remove this device entry", true)? {
                    doc.devices.remove(idx - 1);
                }
            }
            "s" => {
                if let Err(e) = doc.validate() {
                    print_validation_err(&e);
                    continue;
                }
                save_and_offer_load(cfg, &doc)?;
            }
            "" => {}
            _ => println!("Unknown choice."),
        }
    }
}

/// Edit autoprovision policy block.
pub(crate) fn run_autoprovision_wizard(cfg: &TuiConfig) -> Result<(), String> {
    let mut doc = ensure_doc_available(cfg)?;
    let a = &mut doc.autoprovision;
    println!("\n--- Autoprovision policy ---");
    println!(
        "Current: enabled={} rate_limit_per_gateway_per_minute={} pending_ttl_secs={}",
        a.enabled, a.rate_limit_per_gateway_per_minute, a.pending_ttl_secs
    );
    a.enabled = prompt_yes_no(
        "Autoprovision enabled (unknown DevAddr → pending row)",
        a.enabled,
    )?;
    let rl = prompt_with_default(
        "rate_limit_per_gateway_per_minute (0 = unlimited)",
        &a.rate_limit_per_gateway_per_minute.to_string(),
    )?;
    a.rate_limit_per_gateway_per_minute =
        rl.parse().map_err(|e| format!("invalid rate limit: {e}"))?;
    let ttl = prompt_with_default("pending_ttl_secs", &a.pending_ttl_secs.to_string())?;
    a.pending_ttl_secs = ttl.parse().map_err(|e| format!("invalid ttl: {e}"))?;
    if let Err(e) = doc.validate() {
        print_validation_err(&e);
        return Ok(());
    }
    save_and_offer_load(cfg, &doc)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::device_indices_with_application_id;
    use maverick_core::lns_config::{
        ActivationMode, ApplicationEntry, AutoprovisionPolicy, DeviceEntry, LnsConfigDocument,
        OtaaKeys,
    };

    fn sample_doc() -> LnsConfigDocument {
        LnsConfigDocument {
            schema_version: LnsConfigDocument::CURRENT_SCHEMA_VERSION,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: vec![ApplicationEntry {
                id: "app1".to_string(),
                name: "A".to_string(),
                default_region: "EU868".to_string(),
            }],
            devices: vec![
                DeviceEntry {
                    activation_mode: ActivationMode::Otaa,
                    dev_eui: "0102030405060708".to_string(),
                    dev_addr: None,
                    application_id: "app1".to_string(),
                    region: "EU868".to_string(),
                    enabled: true,
                    otaa: Some(OtaaKeys {
                        join_eui: "0000000000000000".to_string(),
                        app_key: "00000000000000000000000000000000".to_string(),
                        nwk_key: None,
                    }),
                    abp: None,
                },
                DeviceEntry {
                    activation_mode: ActivationMode::Otaa,
                    dev_eui: "AABBCCDDEEFF0011".to_string(),
                    dev_addr: None,
                    application_id: "other".to_string(),
                    region: "EU868".to_string(),
                    enabled: true,
                    otaa: Some(OtaaKeys {
                        join_eui: "0000000000000000".to_string(),
                        app_key: "00000000000000000000000000000000".to_string(),
                        nwk_key: None,
                    }),
                    abp: None,
                },
            ],
        }
    }

    #[test]
    fn device_indices_with_application_id_finds_matches() {
        let doc = sample_doc();
        let ix = device_indices_with_application_id(&doc, "app1");
        assert_eq!(ix, vec![0]);
        let ix_other = device_indices_with_application_id(&doc, "other");
        assert_eq!(ix_other, vec![1]);
        assert!(device_indices_with_application_id(&doc, "missing").is_empty());
    }
}
