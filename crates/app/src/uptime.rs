// keep in sync with uptimeColor() in crates/worker/src/lib.rs CLIENT_CONTROLLER
// red(0%) → yellow(50%) → green(100%)
pub fn uptime_color(percent: f32) -> String {
    if percent >= 50.0 {
        let red = ((1.0 - (percent - 50.0) / 50.0) * 255.0).floor() as i32;
        format!("rgb({red}, 255, 0)")
    } else if percent > 0.0 {
        let green = ((percent / 50.0) * 255.0).floor() as i32;
        format!("rgb(255, {green}, 0)")
    } else {
        "rgb(255, 0, 0)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_pure_red() {
        assert_eq!(uptime_color(0.0), "rgb(255, 0, 0)");
    }

    #[test]
    fn fifty_is_pure_yellow() {
        assert_eq!(uptime_color(50.0), "rgb(255, 255, 0)");
    }

    #[test]
    fn seventy_five_is_midway_yellow_to_green() {
        assert_eq!(uptime_color(75.0), "rgb(127, 255, 0)");
    }

    #[test]
    fn hundred_is_pure_green() {
        assert_eq!(uptime_color(100.0), "rgb(0, 255, 0)");
    }

    #[test]
    fn twenty_five_red_to_yellow() {
        assert_eq!(uptime_color(25.0), "rgb(255, 127, 0)");
    }
}
