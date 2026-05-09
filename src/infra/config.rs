#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_id: &'static str,
    pub window_title: &'static str,
    pub default_width: i32,
    pub default_height: i32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_id: "io.vmedia.native-player",
            window_title: "VMedia Native Player",
            default_width: 1280,
            default_height: 800,
        }
    }
}
