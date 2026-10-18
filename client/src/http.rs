use aio_plugin_rbac_model::{AccessControlErrorResponse, AccessControlResponse, AccessControlView};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use wasm_bindgen_futures::JsFuture;

// 直接绑定宿主 SDK：身份由宿主注入，插件不接触会话 Cookie 或会话标识。
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "aioPlugin"], js_name = json, catch)]
    fn json_call(method: &str, path: &str, body: &JsValue) -> Result<js_sys::Promise, JsValue>;
}

pub async fn load() -> Result<AccessControlView, String> {
    json::<AccessControlView>("GET", "/rbac", None::<&()>).await
}

pub async fn save<T: Serialize>(path: &str, body: &T, create: bool) -> Result<(), String> {
    let method = if create { "POST" } else { "PUT" };
    empty(method, path, body).await
}

pub async fn delete(path: &str) -> Result<(), String> {
    json::<serde_json::Value>("DELETE", path, None::<&()>)
        .await
        .map(|_| ())
}

async fn json<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<&impl Serialize>,
) -> Result<T, String> {
    let response = call(method, path, body).await?;
    serde_wasm_bindgen::from_value::<AccessControlResponse<T>>(response)
        .map(|value| value.data)
        .map_err(|error| error.to_string())
}

async fn empty(method: &str, path: &str, body: &impl Serialize) -> Result<(), String> {
    call(method, path, Some(body)).await.map(|_| ())
}

async fn call(method: &str, path: &str, body: Option<&impl Serialize>) -> Result<JsValue, String> {
    let body = body
        .map(|value| value.serialize(&serde_wasm_bindgen::Serializer::json_compatible()))
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(JsValue::UNDEFINED);
    JsFuture::from(json_call(method, path, &body).map_err(js_error)?)
        .await
        .map_err(js_error)
}

fn js_error(error: JsValue) -> String {
    let message = error
        .as_string()
        .or_else(|| {
            js_sys::Reflect::get(&error, &JsValue::from_str("message"))
                .ok()
                .and_then(|value| value.as_string())
        })
        .unwrap_or_else(|| "宿主调用失败".into());
    serde_json::from_str::<AccessControlErrorResponse>(&message)
        .map(|body| body.error)
        .unwrap_or(message)
}
