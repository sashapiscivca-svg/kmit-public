//! Оболонка Tauri для телефону: передає команди інтерфейсу ядру застосунку (`app-core`). Нічого більше.
//! Усі дані лежать у приватній теці застосунку; застосунок не просить доступу до мережі.

use app_core::{App, Ctx};
use serde_json::{json, Value};
use std::sync::Mutex;
use storage::crypto::KdfParams;
use tauri::Manager;

struct Core(Mutex<App>);

/// Android-плагін збереження файлів (вікно «Зберегти як»). На інших платформах його немає.
struct FilesHandle(Option<tauri::plugin::PluginHandle<tauri::Wry>>);

fn files_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::<tauri::Wry, ()>::new("files")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin("ua.kmit.tutor", "FilesPlugin")?;
                app.manage(FilesHandle(Some(handle)));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = api;
                app.manage(FilesHandle(None));
            }
            Ok(())
        })
        .build()
}

/// Озвучення тексту вбудованим синтезатором телефона. Лише Android; помилка, якщо його немає.
#[tauri::command]
async fn speak(files: tauri::State<'_, FilesHandle>, text: String) -> Result<(), String> {
    let Some(handle) = &files.0 else {
        return Err("Озвучення тут не підтримується".to_string());
    };
    handle
        .run_mobile_plugin_async::<serde_json::Value>("speak", json!({"text": text}))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_speaking(files: tauri::State<'_, FilesHandle>) -> Result<(), String> {
    let Some(handle) = &files.0 else {
        return Ok(());
    };
    handle
        .run_mobile_plugin_async::<serde_json::Value>("stopSpeaking", json!({}))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Дозволяє чи забороняє знімки екрана (за замовчуванням заборонено заради приватності).
#[tauri::command]
async fn set_secure(files: tauri::State<'_, FilesHandle>, on: bool) -> Result<(), String> {
    let Some(handle) = &files.0 else {
        return Ok(());
    };
    handle
        .run_mobile_plugin_async::<serde_json::Value>("setSecure", json!({"on": on}))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Ключ профілю, обгорнутий апаратним ключем пристрою (Android Keystore). Лише за feature `keystore`.
#[cfg(feature = "keystore")]
struct KeystoreHook {
    handle: tauri::plugin::PluginHandle<tauri::Wry>,
    slot: u8,
}

#[cfg(feature = "keystore")]
impl KeystoreHook {
    fn call(&self, cmd: &str, data: &[u8]) -> Result<Vec<u8>, storage::StorageError> {
        use storage::StorageError;
        let r: serde_json::Value = self
            .handle
            .run_mobile_plugin(cmd, json!({"slot": self.slot, "data": app_core::base64::encode(data)}))
            .map_err(|e| StorageError::KeyHook(e.to_string()))?;
        let s = r.get("data").and_then(|v| v.as_str()).unwrap_or("");
        if cmd == "destroyKey" {
            return Ok(Vec::new());
        }
        app_core::base64::decode(s).ok_or_else(|| StorageError::KeyHook("хибна відповідь Keystore".to_string()))
    }
}

#[cfg(feature = "keystore")]
impl storage::vault::KeyWrapHook for KeystoreHook {
    fn wrap(&self, data: &[u8]) -> Result<Vec<u8>, storage::StorageError> {
        self.call("wrapKey", data)
    }
    fn unwrap(&self, data: &[u8]) -> Result<Vec<u8>, storage::StorageError> {
        self.call("unwrapKey", data)
    }
    fn destroy(&self) -> Result<(), storage::StorageError> {
        self.call("destroyKey", &[]).map(|_| ())
    }
}

/// Зберігає файл (base64) через системне вікно вибору місця. Повертає `true`, якщо користувач зберіг файл.
#[tauri::command]
async fn save_file(
    files: tauri::State<'_, FilesHandle>,
    name: String,
    mime: String,
    data: String,
) -> Result<bool, String> {
    #[derive(serde::Deserialize)]
    struct Saved {
        saved: bool,
    }
    let Some(handle) = &files.0 else {
        return Err("Збереження файлів тут не підтримується".to_string());
    };
    let r: Saved = handle
        .run_mobile_plugin_async("saveFile", json!({"name": name, "mime": mime, "data": data}))
        .await
        .map_err(|e| e.to_string())?;
    Ok(r.saved)
}

/// Єдина команда: назва, аргументи, поточний час від інтерфейсу. Помилки повертаються як `{error:{code,message}}`.
#[tauri::command(async)]
fn call(core: tauri::State<'_, Core>, name: String, args: Value, ctx: Value) -> Value {
    let Ok(mut app) = core.0.lock() else {
        return json!({"error": {"code": "internal", "message": "Внутрішня помилка"}});
    };
    match app.call(&name, &args, &Ctx::from_json(&ctx)) {
        Ok(v) => v,
        Err(e) => e.to_json(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(files_plugin())
        .setup(|app| {
            let dir = app.path().app_data_dir()?.join("profiles");
            std::fs::create_dir_all(&dir)?;
            // Q25: захист ключа Android Keystore: лише за feature `keystore` (не перевірено на пристрої).
            #[cfg(feature = "keystore")]
            let hook: Option<app_core::HookFactory> = app
                .state::<FilesHandle>()
                .0
                .clone()
                .map(|h| -> app_core::HookFactory {
                    Box::new(move |slot| Box::new(KeystoreHook { handle: h.clone(), slot }))
                });
            #[cfg(not(feature = "keystore"))]
            let hook: Option<app_core::HookFactory> = None;
            app.manage(Core(Mutex::new(App::new(&dir, KdfParams::production(), hook))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![call, save_file, speak, stop_speaking, set_secure])
        .run(tauri::generate_context!())
        .expect("не вдалося запустити застосунок");
}
