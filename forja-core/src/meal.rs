//! Meals: a photo analysed by Claude (or typed by hand) becomes a logged meal with kcal and macros.
//!
//! The core stays offline and deterministic: it builds the request body, reads Claude's answer and
//! stores the meal. The app does the HTTPS call with the user's own API key (kept in the system
//! keychain), sending only the photo — no name, no health data.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const CLAUDE_API_VERSION: &str = "2023-06-01";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";

pub const FOOD_PROMPT: &str = "Eres nutricionista. Analiza la foto de esta comida (contexto: Chile; platos \
típicos como porotos, lentejas, cazuela, pastel de choclo, empanadas, completos, marraqueta, ensalada chilena, \
pollo, arroz, jurel, atún, quesillo, palta). Estima las porciones visibles y responde SOLO con un objeto JSON, \
sin texto adicional, con este formato: {\"plato\": string, \"items\": [{\"nombre\": string, \"gramos\": number, \
\"kcal\": number, \"proteina_g\": number}], \"kcal\": number, \"proteina_g\": number, \"carbohidratos_g\": \
number, \"grasas_g\": number, \"confianza\": \"baja\" | \"media\" | \"alta\", \"nota\": string}. La nota es una \
frase corta y útil en español de Chile. Si la foto no muestra comida, responde {\"plato\": \"no es comida\", \
\"items\": [], \"kcal\": 0, \"proteina_g\": 0, \"carbohidratos_g\": 0, \"grasas_g\": 0, \"confianza\": \"baja\", \
\"nota\": \"\"}.";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MealItem {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "gramos", default)]
    pub grams: Option<f32>,
    #[serde(default)]
    pub kcal: f32,
    #[serde(rename = "proteina_g", default)]
    pub protein_g: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MealEstimate {
    #[serde(rename = "plato")]
    pub dish: String,
    #[serde(default)]
    pub items: Vec<MealItem>,
    #[serde(default)]
    pub kcal: f32,
    #[serde(rename = "proteina_g", default)]
    pub protein_g: f32,
    #[serde(rename = "carbohidratos_g", default)]
    pub carbs_g: f32,
    #[serde(rename = "grasas_g", default)]
    pub fat_g: f32,
    #[serde(rename = "confianza", default)]
    pub confidence: String,
    #[serde(rename = "nota", default)]
    pub note: String,
}

/// Request body for the Messages API with one image and the food prompt.
pub fn claude_request_body(model: &str, media_type: &str, image_base64: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "image", "source": {"type": "base64", "media_type": media_type, "data": image_base64}},
                {"type": "text", "text": FOOD_PROMPT}
            ]
        }]
    })
}

/// The text of a Messages API response, or its error message.
pub fn text_from_response(v: &serde_json::Value) -> Result<String, String> {
    if let Some(e) = v.get("error") {
        let msg = e
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("error desconocido");
        return Err(format!("Claude respondió con un error: {msg}"));
    }
    let text: String = v
        .get("content")
        .and_then(|c| c.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter(|p| p.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        Err("La respuesta de Claude vino vacía.".into())
    } else {
        Ok(text)
    }
}

/// Reads Claude's answer (tolerates code fences or text around the JSON) and checks it makes sense.
pub fn parse_estimate(text: &str) -> Result<MealEstimate, String> {
    let start = text
        .find('{')
        .ok_or("La respuesta no trae datos de la comida.")?;
    let end = text
        .rfind('}')
        .ok_or("La respuesta no trae datos de la comida.")?;
    if end <= start {
        return Err("La respuesta no trae datos de la comida.".into());
    }
    let mut e: MealEstimate = serde_json::from_str(&text[start..=end])
        .map_err(|err| format!("No pude leer la respuesta: {err}"))?;
    if e.dish.trim().eq_ignore_ascii_case("no es comida") {
        return Err("La foto no parece comida. Prueba con otra.".into());
    }
    if e.kcal <= 0.0 && !e.items.is_empty() {
        e.kcal = e.items.iter().map(|i| i.kcal.max(0.0)).sum();
    }
    if e.protein_g <= 0.0 && !e.items.is_empty() {
        e.protein_g = e.items.iter().map(|i| i.protein_g.max(0.0)).sum();
    }
    if !(1.0..=5000.0).contains(&e.kcal) {
        return Err(format!(
            "Las calorías estimadas ({}) no son creíbles.",
            e.kcal.round()
        ));
    }
    for v in [&mut e.protein_g, &mut e.carbs_g, &mut e.fat_g] {
        *v = v.clamp(0.0, 400.0);
    }
    if e.confidence.trim().is_empty() {
        e.confidence = "media".into();
    }
    Ok(e)
}

/// A logged meal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Meal {
    pub id: i64,
    pub date: NaiveDate,
    pub at: DateTime<Utc>,
    pub description: String,
    pub kcal: u32,
    pub protein_g: u32,
    pub carbs_g: u32,
    pub fat_g: u32,
    /// "claude_photo" or "manual".
    pub source: String,
    pub confidence: String,
    /// Path of the photo on this device (the image never enters the database).
    pub photo_ref: Option<String>,
    /// The estimate as JSON, for the history.
    pub detail: String,
}

impl Meal {
    pub fn from_estimate(
        date: NaiveDate,
        at: DateTime<Utc>,
        e: &MealEstimate,
        photo_ref: Option<String>,
    ) -> Self {
        Meal {
            id: 0,
            date,
            at,
            description: e.dish.trim().to_string(),
            kcal: e.kcal.round() as u32,
            protein_g: e.protein_g.round() as u32,
            carbs_g: e.carbs_g.round() as u32,
            fat_g: e.fat_g.round() as u32,
            source: "claude_photo".into(),
            confidence: e.confidence.clone(),
            photo_ref,
            detail: serde_json::to_string(e).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &str = r#"```json
{"plato": "Porotos con riendas", "items": [{"nombre": "porotos", "gramos": 250, "kcal": 310, "proteina_g": 19},
{"nombre": "tallarines", "gramos": 80, "kcal": 120, "proteina_g": 4}], "kcal": 430, "proteina_g": 23,
"carbohidratos_g": 70, "grasas_g": 6, "confianza": "media", "nota": "Buena fuente de fibra."}
```"#;

    #[test]
    fn parses_claude_answer_with_fences() {
        let e = parse_estimate(ANSWER).unwrap();
        assert_eq!(e.dish, "Porotos con riendas");
        assert_eq!(e.kcal, 430.0);
        assert_eq!(e.items.len(), 2);
        let m = Meal::from_estimate(
            NaiveDate::from_ymd_opt(2026, 11, 27).unwrap(),
            Utc::now(),
            &e,
            Some("C:/fotos/almuerzo.jpg".into()),
        );
        assert_eq!((m.kcal, m.protein_g, m.carbs_g, m.fat_g), (430, 23, 70, 6));
    }

    #[test]
    fn fills_totals_from_items_and_rejects_nonsense() {
        let e = parse_estimate(r#"{"plato": "Huevos", "items": [{"nombre": "huevo", "kcal": 78, "proteina_g": 6}, {"nombre": "huevo", "kcal": 78, "proteina_g": 6}]}"#).unwrap();
        assert_eq!(e.kcal, 156.0);
        assert_eq!(e.protein_g, 12.0);
        assert_eq!(e.confidence, "media");
        assert!(parse_estimate(r#"{"plato": "no es comida", "kcal": 0}"#).is_err());
        assert!(parse_estimate("no sé").is_err());
        assert!(parse_estimate(r#"{"plato": "Torta", "kcal": 90000}"#).is_err());
    }

    #[test]
    fn request_and_response_shapes() {
        let body = claude_request_body(DEFAULT_MODEL, "image/jpeg", "AAAA");
        assert_eq!(
            body["messages"][0]["content"][0]["source"]["media_type"],
            "image/jpeg"
        );
        assert_eq!(body["messages"][0]["content"][1]["text"], FOOD_PROMPT);
        let ok = serde_json::json!({"content": [{"type": "text", "text": "hola"}]});
        assert_eq!(text_from_response(&ok).unwrap(), "hola");
        let err = serde_json::json!({"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}});
        assert!(text_from_response(&err)
            .unwrap_err()
            .contains("invalid x-api-key"));
    }
}
