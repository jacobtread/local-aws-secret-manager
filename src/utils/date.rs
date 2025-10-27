use chrono::{DateTime, Utc};

/// Turn the provided DateTime into a f64 representing the seconds with fractional
/// seconds for the sub-second milliseconds
pub fn datetime_to_f64(dt: DateTime<Utc>) -> f64 {
    let seconds = dt.timestamp() as f64;
    let millis = dt.timestamp_subsec_millis() as f64 / 1000.0;
    seconds + millis
}
