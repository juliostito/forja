//! Meal photo → Claude → estimate. Runs on a background thread so the window never freezes.
//! Only the photo (downscaled JPEG) is sent; the API key lives in the Windows Credential Manager
//! (or the ANTHROPIC_API_KEY environment variable).

use base64::Engine;
use forja_core::meal::{
    claude_request_body, parse_estimate, text_from_response, MealEstimate, CLAUDE_API_URL,
    CLAUDE_API_VERSION, DEFAULT_MODEL,
};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

const SERVICE: &str = "forja";
const ACCOUNT: &str = "anthropic_api_key";

pub type Outcome = Result<(MealEstimate, String), String>;

pub fn api_key() -> Option<String> {
    if let Ok(k) = std::env::var("ANTHROPIC_API_KEY") {
        if !k.trim().is_empty() {
            return Some(k.trim().to_string());
        }
    }
    keyring::Entry::new(SERVICE, ACCOUNT)
        .ok()?
        .get_password()
        .ok()
        .filter(|k| !k.trim().is_empty())
}

pub fn save_key(key: &str) -> Result<(), String> {
    keyring::Entry::new(SERVICE, ACCOUNT)
        .and_then(|e| e.set_password(key.trim()))
        .map_err(|e| format!("No pude guardar la clave: {e}"))
}

/// Native file picker for the photo.
pub fn pick_photo() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Foto de tu comida")
        .add_filter("Fotos", &["jpg", "jpeg", "png", "webp"])
        .pick_file()
}

/// Starts the analysis in the background; the receiver gets one outcome.
pub fn analyze(path: PathBuf, key: String) -> Receiver<Outcome> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let shown = path.display().to_string();
        let _ = tx.send(run(&path, &key).map(|e| (e, shown)));
    });
    rx
}

fn run(path: &std::path::Path, key: &str) -> Result<MealEstimate, String> {
    let img = image::open(path)
        .map_err(|e| format!("No pude abrir la foto ({e}). Usa JPG, PNG o WebP."))?;
    let small = image::DynamicImage::ImageRgb8(img.thumbnail(1280, 1280).to_rgb8());
    let mut jpeg = Vec::new();
    small
        .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut jpeg, 85,
        ))
        .map_err(|e| format!("No pude preparar la foto: {e}"))?;
    let data = base64::engine::general_purpose::STANDARD.encode(&jpeg);
    let model = std::env::var("FORJA_CLAUDE_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let body = claude_request_body(&model, "image/jpeg", &data);
    let response = ureq::post(CLAUDE_API_URL)
        .set("x-api-key", key)
        .set("anthropic-version", CLAUDE_API_VERSION)
        .set("content-type", "application/json")
        .timeout(Duration::from_secs(120))
        .send_json(body);
    let value: serde_json::Value = match response {
        Ok(r) => r
            .into_json()
            .map_err(|e| format!("Respuesta ilegible: {e}"))?,
        Err(ureq::Error::Status(code, r)) => {
            let v: serde_json::Value = r.into_json().unwrap_or_default();
            return Err(text_from_response(&v)
                .err()
                .unwrap_or_else(|| format!("Claude respondió el error {code}.")));
        }
        Err(e) => return Err(format!("Sin conexión con Claude: {e}")),
    };
    let text = text_from_response(&value)?;
    parse_estimate(&text)
}
