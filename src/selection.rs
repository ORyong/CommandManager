use crate::model::Command;

#[derive(Default, Debug)]
pub struct Selection {
    pub ids: Vec<String>,
}
impl Selection {
    pub fn toggle(&mut self, id: &str) {
        if self.ids.iter().any(|i| i == id) {
            self.ids.retain(|i| i != id);
        } else {
            self.ids.push(id.into());
        }
    }
    pub fn ordinal(&self, id: &str) -> Option<usize> {
        self.ids.iter().position(|i| i == id).map(|i| i + 1)
    }
    pub fn text(&self, commands: &[Command]) -> anyhow::Result<String> {
        let mut bodies = Vec::new();
        for id in &self.ids {
            let command = commands
                .iter()
                .find(|c| &c.id == id)
                .ok_or_else(|| anyhow::anyhow!("所选命令已被删除，请重新选择"))?;
            bodies.push(command.command.trim_end_matches(['\r', '\n']));
        }
        anyhow::ensure!(!bodies.is_empty(), "请先选择命令");
        Ok(bodies.join("\r\n"))
    }
}
