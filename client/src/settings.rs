use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub antialias: bool,
    pub render_terrain_textures: bool,
    pub render_waves: bool,
    pub render_foam: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            antialias: true,
            render_terrain_textures: true,
            render_waves: true,
            render_foam: true,
        }
    }
}
