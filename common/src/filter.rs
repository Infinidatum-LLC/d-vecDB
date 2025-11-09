use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Filter conditions for payload filtering (Qdrant-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    /// All conditions must be satisfied (AND)
    Must(Vec<Condition>),
    /// At least one condition must be satisfied (OR)
    Should(Vec<Condition>),
    /// None of the conditions must be satisfied (NOT)
    MustNot(Vec<Condition>),
    /// Minimum number of should conditions that must match
    MinShould {
        conditions: Vec<Condition>,
        min_count: usize,
    },
}

/// Individual filter condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    /// Match a specific field value
    Match(FieldCondition),
    /// Nested filter for complex queries
    Filter(Box<Filter>),
}

/// Field-level conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldCondition {
    /// Match exact key-value
    MatchKeyword(MatchKeyword),
    /// Match any value in a list
    MatchAny(MatchAny),
    /// Match text (full-text search)
    MatchText(MatchText),
    /// Range condition (numbers, dates)
    Range(RangeCondition),
    /// Geographic radius search
    GeoRadius(GeoRadius),
    /// Geographic bounding box
    GeoBoundingBox(GeoBoundingBox),
    /// Values within a set
    ValuesCount(ValuesCount),
    /// Check if field exists
    IsEmpty(IsEmpty),
    /// Check if field is null
    IsNull(IsNull),
}

/// Match exact keyword value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchKeyword {
    pub key: String,
    #[serde(flatten)]
    pub value: MatchValue,
}

/// Match value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatchValue {
    Keyword(String),
    Integer(i64),
    Bool(bool),
}

/// Match any value in a list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAny {
    pub key: String,
    pub any: Vec<MatchValue>,
}

/// Full-text match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchText {
    pub key: String,
    pub text: String,
}

/// Numeric/date range condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeCondition {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<f64>,
}

/// Geographic radius search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoRadius {
    pub key: String,
    pub latitude: f64,
    pub longitude: f64,
    pub radius_meters: f64,
}

/// Geographic bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoBoundingBox {
    pub key: String,
    pub top_left: GeoPoint,
    pub bottom_right: GeoPoint,
}

/// Geographic point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
}

/// Count values in array field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuesCount {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gte: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gt: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lte: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<usize>,
}

/// Check if field is empty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsEmpty {
    pub key: String,
}

/// Check if field is null
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsNull {
    pub key: String,
}

/// Evaluate filter against metadata
pub fn evaluate_filter(
    filter: &Filter,
    metadata: &Option<HashMap<String, serde_json::Value>>,
) -> bool {
    let metadata = match metadata {
        Some(m) => m,
        None => return false,
    };

    match filter {
        Filter::Must(conditions) => {
            conditions.iter().all(|c| evaluate_condition(c, metadata))
        }
        Filter::Should(conditions) => {
            conditions.iter().any(|c| evaluate_condition(c, metadata))
        }
        Filter::MustNot(conditions) => {
            !conditions.iter().any(|c| evaluate_condition(c, metadata))
        }
        Filter::MinShould { conditions, min_count } => {
            let matches = conditions
                .iter()
                .filter(|c| evaluate_condition(c, metadata))
                .count();
            matches >= *min_count
        }
    }
}

/// Evaluate a single condition
fn evaluate_condition(
    condition: &Condition,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    match condition {
        Condition::Match(field_cond) => evaluate_field_condition(field_cond, metadata),
        Condition::Filter(filter) => evaluate_filter(filter, &Some(metadata.clone())),
    }
}

/// Evaluate field condition
fn evaluate_field_condition(
    condition: &FieldCondition,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    match condition {
        FieldCondition::MatchKeyword(match_kw) => {
            evaluate_match_keyword(match_kw, metadata)
        }
        FieldCondition::MatchAny(match_any) => {
            evaluate_match_any(match_any, metadata)
        }
        FieldCondition::MatchText(match_text) => {
            evaluate_match_text(match_text, metadata)
        }
        FieldCondition::Range(range) => {
            evaluate_range(range, metadata)
        }
        FieldCondition::GeoRadius(geo) => {
            evaluate_geo_radius(geo, metadata)
        }
        FieldCondition::GeoBoundingBox(geo) => {
            evaluate_geo_bounding_box(geo, metadata)
        }
        FieldCondition::ValuesCount(values_count) => {
            evaluate_values_count(values_count, metadata)
        }
        FieldCondition::IsEmpty(is_empty) => {
            evaluate_is_empty(is_empty, metadata)
        }
        FieldCondition::IsNull(is_null) => {
            evaluate_is_null(is_null, metadata)
        }
    }
}

fn evaluate_match_keyword(
    match_kw: &MatchKeyword,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&match_kw.key) {
        match &match_kw.value {
            MatchValue::Keyword(kw) => {
                if let Some(v) = value.as_str() {
                    return v == kw;
                }
            }
            MatchValue::Integer(i) => {
                if let Some(v) = value.as_i64() {
                    return v == *i;
                }
            }
            MatchValue::Bool(b) => {
                if let Some(v) = value.as_bool() {
                    return v == *b;
                }
            }
        }
    }
    false
}

fn evaluate_match_any(
    match_any: &MatchAny,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&match_any.key) {
        for match_val in &match_any.any {
            match match_val {
                MatchValue::Keyword(kw) => {
                    if let Some(v) = value.as_str() {
                        if v == kw {
                            return true;
                        }
                    }
                }
                MatchValue::Integer(i) => {
                    if let Some(v) = value.as_i64() {
                        if v == *i {
                            return true;
                        }
                    }
                }
                MatchValue::Bool(b) => {
                    if let Some(v) = value.as_bool() {
                        if v == *b {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn evaluate_match_text(
    match_text: &MatchText,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&match_text.key) {
        if let Some(text) = value.as_str() {
            // Simple case-insensitive substring match
            // TODO: Implement proper full-text search with tokenization
            return text.to_lowercase().contains(&match_text.text.to_lowercase());
        }
    }
    false
}

fn evaluate_range(
    range: &RangeCondition,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&range.key) {
        let num = if let Some(n) = value.as_f64() {
            n
        } else if let Some(n) = value.as_i64() {
            n as f64
        } else {
            return false;
        };

        if let Some(gte) = range.gte {
            if num < gte {
                return false;
            }
        }
        if let Some(gt) = range.gt {
            if num <= gt {
                return false;
            }
        }
        if let Some(lte) = range.lte {
            if num > lte {
                return false;
            }
        }
        if let Some(lt) = range.lt {
            if num >= lt {
                return false;
            }
        }
        return true;
    }
    false
}

fn evaluate_geo_radius(
    geo: &GeoRadius,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&geo.key) {
        if let Some(obj) = value.as_object() {
            let lat = obj.get("lat").and_then(|v| v.as_f64());
            let lon = obj.get("lon").and_then(|v| v.as_f64());

            if let (Some(lat), Some(lon)) = (lat, lon) {
                let distance = haversine_distance(
                    geo.latitude,
                    geo.longitude,
                    lat,
                    lon,
                );
                return distance <= geo.radius_meters;
            }
        }
    }
    false
}

fn evaluate_geo_bounding_box(
    geo: &GeoBoundingBox,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&geo.key) {
        if let Some(obj) = value.as_object() {
            let lat = obj.get("lat").and_then(|v| v.as_f64());
            let lon = obj.get("lon").and_then(|v| v.as_f64());

            if let (Some(lat), Some(lon)) = (lat, lon) {
                return lat <= geo.top_left.lat
                    && lat >= geo.bottom_right.lat
                    && lon >= geo.top_left.lon
                    && lon <= geo.bottom_right.lon;
            }
        }
    }
    false
}

fn evaluate_values_count(
    values_count: &ValuesCount,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&values_count.key) {
        if let Some(arr) = value.as_array() {
            let count = arr.len();

            if let Some(gte) = values_count.gte {
                if count < gte {
                    return false;
                }
            }
            if let Some(gt) = values_count.gt {
                if count <= gt {
                    return false;
                }
            }
            if let Some(lte) = values_count.lte {
                if count > lte {
                    return false;
                }
            }
            if let Some(lt) = values_count.lt {
                if count >= lt {
                    return false;
                }
            }
            return true;
        }
    }
    false
}

fn evaluate_is_empty(
    is_empty: &IsEmpty,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    if let Some(value) = metadata.get(&is_empty.key) {
        if let Some(arr) = value.as_array() {
            return arr.is_empty();
        }
        if let Some(s) = value.as_str() {
            return s.is_empty();
        }
    }
    false
}

fn evaluate_is_null(
    is_null: &IsNull,
    metadata: &HashMap<String, serde_json::Value>,
) -> bool {
    metadata.get(&is_null.key).map_or(false, |v| v.is_null())
}

/// Calculate haversine distance between two geo points (in meters)
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6371000.0; // meters

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_keyword() {
        let mut metadata = HashMap::new();
        metadata.insert("category".to_string(), serde_json::json!("electronics"));

        let filter = Filter::Must(vec![Condition::Match(FieldCondition::MatchKeyword(
            MatchKeyword {
                key: "category".to_string(),
                value: MatchValue::Keyword("electronics".to_string()),
            },
        ))]);

        assert!(evaluate_filter(&filter, &Some(metadata)));
    }

    #[test]
    fn test_range_condition() {
        let mut metadata = HashMap::new();
        metadata.insert("price".to_string(), serde_json::json!(50.0));

        let filter = Filter::Must(vec![Condition::Match(FieldCondition::Range(
            RangeCondition {
                key: "price".to_string(),
                gte: Some(10.0),
                gt: None,
                lte: Some(100.0),
                lt: None,
            },
        ))]);

        assert!(evaluate_filter(&filter, &Some(metadata)));
    }

    #[test]
    fn test_must_not() {
        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), serde_json::json!("active"));

        let filter = Filter::MustNot(vec![Condition::Match(FieldCondition::MatchKeyword(
            MatchKeyword {
                key: "status".to_string(),
                value: MatchValue::Keyword("inactive".to_string()),
            },
        ))]);

        assert!(evaluate_filter(&filter, &Some(metadata)));
    }

    #[test]
    fn test_geo_radius() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "location".to_string(),
            serde_json::json!({
                "lat": 40.7128,
                "lon": -74.0060
            }),
        );

        let filter = Filter::Must(vec![Condition::Match(FieldCondition::GeoRadius(
            GeoRadius {
                key: "location".to_string(),
                latitude: 40.7128,
                longitude: -74.0060,
                radius_meters: 1000.0,
            },
        ))]);

        assert!(evaluate_filter(&filter, &Some(metadata)));
    }
}
