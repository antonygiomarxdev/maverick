//! `maverick-edge config …` — declarative LNS file + SQLite sync.

use std::fs;
use std::path::PathBuf;

use maverick_adapter_persistence_sqlite::SqlitePersistence;
use maverick_core::lns_config::LnsConfigDocument;
use maverick_domain::RegionId;
use serde::Serialize;

use crate::cli_constants::LNS_CONFIG_TEMPLATE;
use crate::commands::sqlite_opts;
use crate::paths::db_path;
use crate::probe::HardwareCapabilities;

#[derive(Serialize)]
struct ConfigInitResponse {
    path: String,
    written: bool,
}

#[derive(Serialize)]
struct ConfigValidateResponse {
    path: String,
    ok: bool,
}

#[derive(Serialize)]
struct ConfigLoadResponse {
    path: String,
    applications: usize,
    devices: usize,
    ok: bool,
}

#[derive(Serialize)]
struct ConfigShowResponse {
    autoprovision: maverick_adapter_persistence_sqlite::LnsAutoprovisionMeta,
    applications: Vec<maverick_adapter_persistence_sqlite::LnsApplicationRow>,
    devices: Vec<maverick_adapter_persistence_sqlite::LnsDeviceListRow>,
    pending: Vec<maverick_adapter_persistence_sqlite::LnsPendingRow>,
}

fn print_json<T: Serialize>(v: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(v).expect("config json serialize")
    );
}

pub(crate) fn run_config_init(config_path: PathBuf, force: bool) {
    if config_path.exists() && !force {
        eprintln!(
            "config file already exists: {}. Pass --force to overwrite.",
            config_path.display()
        );
        std::process::exit(2);
    }
    if let Some(parent) = config_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("failed to create parent dir {}: {e}", parent.display());
            std::process::exit(1);
        }
    }
    match fs::write(&config_path, LNS_CONFIG_TEMPLATE) {
        Ok(()) => {
            print_json(&ConfigInitResponse {
                path: config_path.display().to_string(),
                written: true,
            });
        }
        Err(e) => {
            eprintln!("failed to write {}: {e}", config_path.display());
            std::process::exit(1);
        }
    }
}

pub(crate) fn run_config_validate(config_path: PathBuf) {
    let raw = match fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {e}", config_path.display());
            std::process::exit(1);
        }
    };
    let doc: LnsConfigDocument = match toml::from_str(&raw) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("TOML parse error in {}: {e}", config_path.display());
            std::process::exit(1);
        }
    };
    if let Err(msg) = doc.validate() {
        eprintln!("validation failed: {msg}");
        std::process::exit(1);
    }
    print_json(&ConfigValidateResponse {
        path: config_path.display().to_string(),
        ok: true,
    });
}

pub(crate) fn run_config_load(data_dir: PathBuf, db_file: &str, config_path: PathBuf) {
    let raw = match fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {e}", config_path.display());
            std::process::exit(1);
        }
    };
    let doc: LnsConfigDocument = match toml::from_str(&raw) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("TOML parse error: {e}");
            std::process::exit(1);
        }
    };
    if let Err(msg) = doc.validate() {
        eprintln!("validation failed: {msg}");
        std::process::exit(1);
    }
    let dbp = db_path(&data_dir, db_file);
    let cap = HardwareCapabilities::probe();
    let profile = cap.suggested_install_profile();
    let policy = profile.default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("storage open failed: {e}");
            std::process::exit(1);
        }
    };
    let apps = doc.applications.len();
    let devs = doc.devices.len();
    match store.apply_lns_config(&doc) {
        Ok(()) => {
            print_json(&ConfigLoadResponse {
                path: config_path.display().to_string(),
                applications: apps,
                devices: devs,
                ok: true,
            });
        }
        Err(e) => {
            eprintln!("config load failed: {e}");
            std::process::exit(1);
        }
    }
}

pub(crate) fn run_config_show(data_dir: PathBuf, db_file: &str) {
    let dbp = db_path(&data_dir, db_file);
    if !dbp.exists() {
        eprintln!(
            "database not found at {}. Run setup or create the data directory first.",
            dbp.display()
        );
        std::process::exit(1);
    }
    let cap = HardwareCapabilities::probe();
    let profile = cap.suggested_install_profile();
    let policy = profile.default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("storage open failed: {e}");
            std::process::exit(1);
        }
    };
    let autoprovision = match store.lns_autoprovision_policy() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("read policy: {e}");
            std::process::exit(1);
        }
    };
    let applications = match store.lns_list_applications() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("list applications: {e}");
            std::process::exit(1);
        }
    };
    let devices = match store.lns_list_devices() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("list devices: {e}");
            std::process::exit(1);
        }
    };
    let pending = match store.lns_list_pending() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("list pending: {e}");
            std::process::exit(1);
        }
    };
    print_json(&ConfigShowResponse {
        autoprovision,
        applications,
        devices,
        pending,
    });
}

pub(crate) fn run_config_list_apps(data_dir: PathBuf, db_file: &str) {
    list_only(data_dir, db_file, |s| s.lns_list_applications());
}

pub(crate) fn run_config_list_devices(data_dir: PathBuf, db_file: &str) {
    list_only(data_dir, db_file, |s| s.lns_list_devices());
}

pub(crate) fn run_config_list_pending(data_dir: PathBuf, db_file: &str) {
    list_only(data_dir, db_file, |s| s.lns_list_pending());
}

fn list_only<T: Serialize, F>(data_dir: PathBuf, db_file: &str, f: F)
where
    F: FnOnce(&SqlitePersistence) -> Result<T, maverick_core::error::AppError>,
{
    let dbp = db_path(&data_dir, db_file);
    if !dbp.exists() {
        eprintln!("database not found at {}.", dbp.display());
        std::process::exit(1);
    }
    let cap = HardwareCapabilities::probe();
    let policy = cap.suggested_install_profile().default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("storage open failed: {e}");
            std::process::exit(1);
        }
    };
    match f(&store) {
        Ok(v) => print_json(&v),
        Err(e) => {
            eprintln!("query failed: {e}");
            std::process::exit(1);
        }
    }
}

pub(crate) fn run_config_approve_device(
    data_dir: PathBuf,
    db_file: &str,
    dev_eui: String,
    dev_addr: String,
    application_id: String,
    region: String,
) {
    let region: RegionId = match region.parse() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("invalid region: {e}");
            std::process::exit(1);
        }
    };
    let dbp = db_path(&data_dir, db_file);
    if !dbp.exists() {
        eprintln!("database not found.");
        std::process::exit(1);
    }
    let cap = HardwareCapabilities::probe();
    let policy = cap.suggested_install_profile().default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("storage open failed: {e}");
            std::process::exit(1);
        }
    };
    match store.lns_approve_device(&dev_eui, &dev_addr, &application_id, region) {
        Ok(()) => {
            println!(
                "{}",
                serde_json::json!({ "ok": true, "dev_eui": dev_eui, "dev_addr": dev_addr })
            );
        }
        Err(e) => {
            eprintln!("approve failed: {e}");
            std::process::exit(1);
        }
    }
}

pub(crate) fn run_config_reject_device(data_dir: PathBuf, db_file: &str, dev_addr: String) {
    let addr_u = match maverick_core::lns_config::parse_hex_dev_addr(&dev_addr) {
        Ok(a) => maverick_domain::DevAddr(a),
        Err(e) => {
            eprintln!("invalid dev_addr: {e}");
            std::process::exit(1);
        }
    };
    let dbp = db_path(&data_dir, db_file);
    if !dbp.exists() {
        eprintln!("database not found.");
        std::process::exit(1);
    }
    let cap = HardwareCapabilities::probe();
    let policy = cap.suggested_install_profile().default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("storage open failed: {e}");
            std::process::exit(1);
        }
    };
    match store.lns_delete_pending(addr_u) {
        Ok(()) => {
            println!(
                "{}",
                serde_json::json!({ "ok": true, "dev_addr": dev_addr })
            );
        }
        Err(e) => {
            eprintln!("reject failed: {e}");
            std::process::exit(1);
        }
    }
}
