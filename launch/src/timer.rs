use std::time::SystemTime;

use loki::tracing::warn;
use thousands::Separable;

// pretty-print the duration elapsed since `time` in ms
// and handles error
pub fn duration_since(time: SystemTime) -> String {
    match time.elapsed() {
        Ok(duration) => {
            let milliseconds = duration.as_millis();
            milliseconds.separate_with_underscores()
        }
        Err(err) => {
            warn!("Timer error : {}", err);
            "'timer_error'".to_string()
        }
    }
}
