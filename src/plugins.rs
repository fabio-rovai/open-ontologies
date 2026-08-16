//! WASM plugin host — the in-process community extension surface.
//!
//! Plugins are `.wasm` modules (wasm32-unknown-unknown) discovered from the
//! plugin directories and executed in the wasmi interpreter with fuel
//! metering, so a misbehaving plugin runs out of fuel instead of hanging the
//! server. Plugins are pure JSON → JSON transforms: they import nothing from
//! the host, and any graph context they need (e.g. SPARQL results) is injected
//! into their input by the caller. That keeps the capability model explicit —
//! a plugin can compute over what it is handed, and nothing else.
//!
//! ## ABI v1
//!
//! A plugin exports:
//!
//! - `memory` — its linear memory
//! - `oo_abi_version() -> i32` — must return `1`
//! - `oo_alloc(len: i32) -> i32` — allocate `len` bytes, return the pointer
//! - `oo_describe() -> i64` — return `(ptr << 32) | len` of a UTF-8 JSON
//!   manifest: `{"name", "version", "tools": [{"name", "description"}]}`
//! - `oo_call(ptr: i32, len: i32) -> i64` — input is UTF-8 JSON
//!   `{"tool": ..., "input": ..., "bindings": [...]}`; returns packed ptr/len
//!   of the UTF-8 JSON result
//!
//! See `examples/plugins/` for a Rust reference implementation.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use wasmi::{Config, Engine, Instance, Linker, Module, Store};

/// ABI version this host speaks.
pub const ABI_VERSION: i32 = 1;

/// Fuel budget per plugin invocation. Interpreter instructions, not wall
/// clock; enough for validation-scale work, small enough to stop runaways.
const FUEL_PER_CALL: u64 = 500_000_000;

/// Hard cap on the byte length a plugin may return.
const MAX_RETURN_LEN: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct PluginToolDecl {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub tools: Vec<PluginToolDecl>,
}

/// A discovered plugin: its file plus the manifest it reported.
#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub path: PathBuf,
    pub manifest: PluginManifest,
}

/// Plugin search directories, in priority order:
/// 1. `OPEN_ONTOLOGIES_PLUGIN_DIRS` (colon-separated), if set — exclusively
/// 2. `~/.open-ontologies/plugins` and `./plugins`
pub fn plugin_dirs() -> Vec<PathBuf> {
    if let Ok(v) = std::env::var("OPEN_ONTOLOGIES_PLUGIN_DIRS")
        && !v.trim().is_empty() {
            return v.split(':').map(|p| PathBuf::from(crate::config::expand_tilde(p))).collect();
        }
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        dirs.push(Path::new(&home).join(".open-ontologies").join("plugins"));
    }
    dirs.push(PathBuf::from("plugins"));
    dirs
}

/// Enumerate `.wasm` files in the plugin directories and describe each.
/// A plugin that fails to load or describe is reported as an error string
/// rather than aborting discovery — one broken plugin must not hide the rest.
pub fn discover() -> (Vec<DiscoveredPlugin>, Vec<String>) {
    let mut plugins = Vec::new();
    let mut errors = Vec::new();
    for dir in plugin_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "wasm"))
            .collect();
        paths.sort();
        for path in paths {
            match describe(&path) {
                Ok(manifest) => plugins.push(DiscoveredPlugin { path, manifest }),
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
    }
    (plugins, errors)
}

/// Find a discovered plugin by manifest name.
pub fn find(name: &str) -> Result<DiscoveredPlugin, String> {
    let (plugins, errors) = discover();
    plugins
        .into_iter()
        .find(|p| p.manifest.name == name)
        .ok_or_else(|| {
            let mut msg = format!("plugin '{name}' not found in {:?}", plugin_dirs());
            if !errors.is_empty() {
                msg.push_str(&format!("; broken plugins: {}", errors.join("; ")));
            }
            msg
        })
}

/// Load a plugin and return its manifest.
pub fn describe(path: &Path) -> Result<PluginManifest, String> {
    let mut runtime = PluginRuntime::load(path)?;
    let packed = runtime
        .typed_call::<(), i64>("oo_describe", ())
        .map_err(|e| format!("oo_describe failed: {e}"))?;
    let bytes = runtime.read_packed(packed)?;
    let manifest: PluginManifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("oo_describe returned invalid manifest JSON: {e}"))?;
    if manifest.name.is_empty() {
        return Err("manifest name is empty".to_string());
    }
    Ok(manifest)
}

/// Invoke a tool on a plugin. `payload` is the full input document the plugin
/// receives (`{"tool", "input", "bindings"}` — assembled by the caller).
/// Each call runs in a fresh instance: plugins are stateless by construction.
pub fn call(path: &Path, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut runtime = PluginRuntime::load(path)?;
    let input = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let ptr = runtime
        .typed_call::<i32, i32>("oo_alloc", input.len() as i32)
        .map_err(|e| format!("oo_alloc failed: {e}"))?;
    runtime
        .memory
        .write(&mut runtime.store, ptr as usize, &input)
        .map_err(|e| format!("input write failed: {e}"))?;
    let packed = runtime
        .typed_call::<(i32, i32), i64>("oo_call", (ptr, input.len() as i32))
        .map_err(|e| format!("oo_call failed (out of fuel or trapped): {e}"))?;
    let bytes = runtime.read_packed(packed)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("plugin returned invalid JSON: {e}"))
}

/// One loaded instance: engine, store (with fuel), instance, memory.
struct PluginRuntime {
    store: Store<()>,
    instance: Instance,
    memory: wasmi::Memory,
}

impl PluginRuntime {
    fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &bytes).map_err(|e| format!("invalid wasm: {e}"))?;
        let mut store = Store::new(&engine, ());
        store
            .set_fuel(FUEL_PER_CALL)
            .map_err(|e| format!("fuel setup failed: {e}"))?;
        // Empty linker: ABI v1 plugins import nothing from the host.
        let linker = <Linker<()>>::new(&engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|e| format!("instantiation failed (plugin imports host functions? ABI v1 allows none): {e}"))?;
        let abi = instance
            .get_typed_func::<(), i32>(&store, "oo_abi_version")
            .map_err(|_| "missing export oo_abi_version — not an Open Ontologies plugin".to_string())?
            .call(&mut store, ())
            .map_err(|e| format!("oo_abi_version trapped: {e}"))?;
        if abi != ABI_VERSION {
            return Err(format!("plugin speaks ABI v{abi}, host speaks v{ABI_VERSION}"));
        }
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| "plugin exports no `memory`".to_string())?;
        Ok(Self { store, instance, memory })
    }

    fn typed_call<P, R>(&mut self, name: &str, params: P) -> Result<R, String>
    where
        P: wasmi::WasmParams,
        R: wasmi::WasmResults,
    {
        self.instance
            .get_typed_func::<P, R>(&self.store, name)
            .map_err(|e| format!("missing/mistyped export {name}: {e}"))?
            .call(&mut self.store, params)
            .map_err(|e| e.to_string())
    }

    /// Unpack an `(ptr << 32) | len` return value and copy it out of guest memory.
    fn read_packed(&self, packed: i64) -> Result<Vec<u8>, String> {
        let ptr = (packed as u64 >> 32) as usize;
        let len = (packed as u64 & 0xFFFF_FFFF) as usize;
        if len > MAX_RETURN_LEN {
            return Err(format!("plugin returned {len} bytes, cap is {MAX_RETURN_LEN}"));
        }
        let mut buf = vec![0u8; len];
        self.memory
            .read(&self.store, ptr, &mut buf)
            .map_err(|e| format!("result read out of bounds: {e}"))?;
        Ok(buf)
    }
}
