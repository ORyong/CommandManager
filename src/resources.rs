use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

/// 自带图标叠在组件库资源包之上；本项目的两个 SVG 优先命中。
pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let data: Option<&'static [u8]> = match path {
            "icons/pencil.svg" => Some(include_bytes!("../assets/icons/pencil.svg")),
            "icons/trash-2.svg" => Some(include_bytes!("../assets/icons/trash-2.svg")),
            _ => None,
        };
        if let Some(data) = data {
            return Ok(Some(Cow::Borrowed(data)));
        }
        gpui_component_assets::Assets.load(path)
    }
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut names = gpui_component_assets::Assets.list(path)?;
        names.extend(
            ["icons/pencil.svg", "icons/trash-2.svg"]
                .into_iter()
                .filter(|p| p.starts_with(path))
                .map(SharedString::from),
        );
        Ok(names)
    }
}
