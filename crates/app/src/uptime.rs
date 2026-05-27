// keep in sync with uptimeColor() in crates/worker/src/lib.rs CLIENT_CONTROLLER
pub fn uptime_color(percent: f32) -> String {
    if percent >= 75.0 {
        let green_intensity = (((percent - 75.0) / 25.0) * 255.0).floor() as i32;
        format!("rgb({}, 255, 0)", 255 - green_intensity)
    } else if percent >= 50.0 {
        let ratio = (percent - 50.0) / 25.0;
        format!("rgb(255, 255, {})", (ratio * 255.0).floor() as i32)
    } else if percent > 0.0 {
        let ratio = percent / 50.0;
        format!("rgb(255, {}, 0)", (ratio * 255.0).floor() as i32)
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
    fn seventy_five_is_pure_yellow_green_transition() {
        assert_eq!(uptime_color(75.0), "rgb(255, 255, 0)");
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
