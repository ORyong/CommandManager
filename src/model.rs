use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

pub const BUILTIN_GROUP: &str = "builtin";
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub fn new_id(prefix: &str) -> String {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{prefix}-{time:x}-{:x}",
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub groups: Vec<String>,
}

impl Command {
    pub fn builtin(&self) -> bool {
        self.id.starts_with("builtin:")
    }
    pub fn normalize(&mut self) -> Result<()> {
        self.name = self.name.trim().to_owned();
        ensure!(
            !self.name.is_empty()
                && self.name.chars().count() <= 80
                && !self.name.contains(['\0', '\r', '\n']),
            "名称应为 1–80 字的单行文字"
        );
        ensure!(
            !self.command.trim().is_empty()
                && self.command.chars().count() <= 4000
                && !self.command.contains('\0'),
            "命令应为 1–4000 字，不能包含空字符"
        );
        ensure!(
            self.notes.chars().count() <= 8000 && !self.notes.contains('\0'),
            "备注最多 8000 字，不能包含空字符"
        );
        self.tags = normalize_tags(&self.tags)?;
        let mut seen = BTreeSet::new();
        self.groups.retain(|id| seen.insert(id.clone()));
        Ok(())
    }
}

pub fn normalize_tags(tags: &[String]) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#');
        if tag.is_empty() {
            continue;
        }
        ensure!(
            tag.chars().count() <= 40 && !tag.contains(['\r', '\n', '\0']),
            "标签最多 40 字，不能换行"
        );
        if seen.insert(tag.to_lowercase()) {
            result.push(tag.to_owned());
        }
    }
    ensure!(result.len() <= 20, "每条命令最多 20 个标签");
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// None inherits builtin; Some([]) explicitly means ungrouped.
    #[serde(default)]
    pub groups: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Library {
    pub version: u32,
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub deleted_builtins: BTreeSet<String>,
    #[serde(default)]
    pub builtin_metadata: BTreeMap<String, Metadata>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 2,
            commands: vec![],
            groups: vec![],
            tags: vec![],
            deleted_builtins: BTreeSet::new(),
            builtin_metadata: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize)]
struct Recipe {
    id: String,
    name: String,
    command: String,
    platforms: Vec<String>,
}

pub fn builtin_templates() -> Vec<Command> {
    let platform = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "Mac"
    } else {
        "Posix"
    };
    let recipes: Vec<Recipe> = serde_json::from_str(include_str!("../assets/catalog.json"))
        .expect("embedded, validated builtin recipes");
    recipes
        .into_iter()
        .filter(|recipe| recipe.platforms.iter().any(|p| p == platform))
        .map(|r| Command {
            id: r.id,
            name: r.name,
            command: r.command,
            groups: vec![BUILTIN_GROUP.into()],
            ..Default::default()
        })
        .collect()
}

pub fn all_builtin_ids() -> BTreeSet<String> {
    let recipes: Vec<Recipe> =
        serde_json::from_str(include_str!("../assets/catalog.json")).expect("embedded recipes");
    recipes.into_iter().map(|r| r.id).collect()
}

impl Library {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut value: serde_json::Value =
            serde_json::from_slice(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))?;
        let version = value.get("version").and_then(|v| v.as_u64());
        ensure!(matches!(version, Some(1 | 2)), "不支持的命令库版本");
        // Also accept the unpublished prototype's organization-shaped v2.
        if let Some(organization) = value.get("organization").cloned() {
            ensure!(organization.is_object(), "分组数据格式错误");
            if let Some(metadata) = value.get("builtin_metadata") {
                ensure!(metadata.is_object(), "内置备注格式错误");
            }
            value["groups"] = organization
                .get("groups")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            let membership = organization.get("membership").and_then(|m| m.as_object());
            if let Some(commands) = value.get_mut("commands").and_then(|c| c.as_array_mut()) {
                for command in commands {
                    let groups = membership.and_then(|m| {
                        command
                            .get("id")
                            .and_then(|i| i.as_str())
                            .and_then(|id| m.get(id))
                    });
                    command["groups"] = match groups {
                        Some(serde_json::Value::String(group)) => serde_json::json!([group]),
                        Some(serde_json::Value::Array(groups)) => serde_json::json!(groups),
                        _ => serde_json::json!([]),
                    };
                }
            }
            if let Some(membership) = membership {
                for (id, group) in membership
                    .iter()
                    .filter(|(id, _)| id.starts_with("builtin:"))
                {
                    if value.get("builtin_metadata").is_none() {
                        value["builtin_metadata"] = serde_json::json!({});
                    }
                    if value["builtin_metadata"].get(id).is_none() {
                        value["builtin_metadata"][id] = serde_json::json!({});
                    }
                    ensure!(
                        value["builtin_metadata"][id].is_object(),
                        "内置备注格式错误"
                    );
                    value["builtin_metadata"][id]["groups"] = match group {
                        serde_json::Value::String(group) => serde_json::json!([group]),
                        serde_json::Value::Array(groups) => serde_json::json!(groups),
                        _ => serde_json::json!([]),
                    };
                }
            }
        }
        value["version"] = serde_json::json!(2);
        let mut library: Self = serde_json::from_value(value)?;
        library.validate()?;
        Ok(library)
    }

    pub fn visible_commands(&self) -> Vec<Command> {
        let mut commands = self.commands.clone();
        for mut command in builtin_templates() {
            if self.deleted_builtins.contains(&command.id) {
                continue;
            }
            if let Some(meta) = self.builtin_metadata.get(&command.id) {
                command.notes = meta.notes.clone();
                command.tags = meta.tags.clone();
                if let Some(groups) = &meta.groups {
                    command.groups = groups.clone();
                }
            }
            commands.push(command);
        }
        commands
    }

    pub fn validate(&mut self) -> Result<()> {
        ensure!(self.version == 2, "不支持的命令库版本");
        ensure!(
            self.commands.len() <= 10000 && self.groups.len() <= 1000,
            "命令库过大"
        );
        let mut groups = BTreeSet::from([BUILTIN_GROUP.to_owned()]);
        let mut names = BTreeSet::new();
        for group in &mut self.groups {
            group.name = group.name.trim().to_owned();
            ensure!(
                !group.id.is_empty() && group.id.len() <= 80 && groups.insert(group.id.clone()),
                "分组 ID 无效或重复"
            );
            ensure!(
                !group.name.is_empty()
                    && group.name.chars().count() <= 80
                    && !group.name.contains(['\0', '\r', '\n'])
                    && names.insert(group.name.to_lowercase()),
                "分组名称为空、过长或重复"
            );
        }
        let mut ids = BTreeSet::new();
        for command in &mut self.commands {
            ensure!(
                !command.id.is_empty()
                    && command.id.len() <= 80
                    && !command.builtin()
                    && ids.insert(command.id.clone()),
                "命令 ID 无效或重复"
            );
            command.normalize()?;
            ensure!(
                command.groups.iter().all(|g| groups.contains(g)),
                "命令引用了不存在的分组"
            );
        }
        for (id, meta) in &mut self.builtin_metadata {
            ensure!(
                id.starts_with("builtin:") && id.len() > 8 && id.len() <= 80,
                "无效内置命令 ID"
            );
            let mut command = Command {
                name: "builtin".into(),
                command: "builtin".into(),
                notes: meta.notes.clone(),
                tags: meta.tags.clone(),
                groups: meta.groups.clone().unwrap_or_default(),
                ..Default::default()
            };
            command.normalize()?;
            ensure!(
                command.groups.iter().all(|g| groups.contains(g)),
                "内置命令引用了不存在的分组"
            );
            meta.tags = command.tags;
        }
        ensure!(
            self.deleted_builtins
                .iter()
                .all(|id| id.starts_with("builtin:") && id.len() > 8 && id.len() <= 80),
            "无效内置删除记录"
        );
        let mut tags = normalize_tags_unbounded(&self.tags)?;
        for command in self.visible_commands() {
            for tag in command.tags {
                if !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
                    tags.push(tag);
                }
            }
        }
        self.tags = tags;
        Ok(())
    }

    pub fn save(&mut self, mut command: Command) -> Result<()> {
        command.normalize()?;
        if command.builtin() {
            let template = builtin_templates()
                .into_iter()
                .find(|c| c.id == command.id)
                .ok_or_else(|| anyhow::anyhow!("内置命令不存在"))?;
            ensure!(
                template.name == command.name && template.command == command.command,
                "内置正文请另存副本后修改"
            );
            self.builtin_metadata.insert(
                command.id,
                Metadata {
                    notes: command.notes,
                    tags: command.tags,
                    groups: Some(command.groups),
                },
            );
        } else {
            if command.id.is_empty() {
                command.id = new_id("cmd");
            }
            if let Some(existing) = self.commands.iter_mut().find(|c| c.id == command.id) {
                *existing = command;
            } else {
                self.commands.push(command);
            }
        }
        self.validate()
    }

    pub fn remove(&mut self, ids: &[String]) {
        self.commands.retain(|c| !ids.contains(&c.id));
        self.deleted_builtins
            .extend(ids.iter().filter(|id| id.starts_with("builtin:")).cloned());
    }

    pub fn add_group(&mut self, name: &str) -> Result<()> {
        self.groups.push(Group {
            id: new_id("group"),
            name: name.into(),
        });
        self.validate()
    }

    pub fn rename_group(&mut self, id: &str, name: &str) -> Result<()> {
        let group = self
            .groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or_else(|| anyhow::anyhow!("分组已不存在"))?;
        group.name = name.into();
        self.validate()
    }

    pub fn remove_group(&mut self, id: &str) -> Result<()> {
        ensure!(id != BUILTIN_GROUP, "内置分组请使用清空内置命令");
        self.groups.retain(|g| g.id != id);
        for command in &mut self.commands {
            command.groups.retain(|g| g != id);
        }
        for meta in self.builtin_metadata.values_mut() {
            if let Some(groups) = &mut meta.groups {
                groups.retain(|g| g != id);
            }
        }
        Ok(())
    }

    pub fn rename_tag(&mut self, old: &str, new: &str) -> Result<()> {
        let normalized = normalize_tags(&[new.into()])?;
        ensure!(!normalized.is_empty(), "标签不能为空");
        let new = normalized[0].clone();
        if old.to_lowercase() != new.to_lowercase()
            && self
                .tags
                .iter()
                .any(|tag| tag.to_lowercase() == new.to_lowercase())
        {
            bail!("该标签已存在");
        }
        for tag in &mut self.tags {
            if tag.to_lowercase() == old.to_lowercase() {
                *tag = new.clone();
            }
        }
        for command in &mut self.commands {
            for tag in &mut command.tags {
                if tag.to_lowercase() == old.to_lowercase() {
                    *tag = new.clone();
                }
            }
        }
        for meta in self.builtin_metadata.values_mut() {
            for tag in &mut meta.tags {
                if tag.to_lowercase() == old.to_lowercase() {
                    *tag = new.clone();
                }
            }
        }
        self.validate()
    }

    pub fn remove_tag(&mut self, name: &str) {
        self.tags
            .retain(|tag| tag.to_lowercase() != name.to_lowercase());
        for command in &mut self.commands {
            command
                .tags
                .retain(|tag| tag.to_lowercase() != name.to_lowercase());
        }
        for meta in self.builtin_metadata.values_mut() {
            meta.tags
                .retain(|tag| tag.to_lowercase() != name.to_lowercase());
        }
    }
}

pub fn normalize_tags_unbounded(tags: &[String]) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for tag in tags {
        for normalized in normalize_tags(std::slice::from_ref(tag))? {
            if !result
                .iter()
                .any(|t: &String| t.to_lowercase() == normalized.to_lowercase())
            {
                result.push(normalized);
            }
        }
    }
    Ok(result)
}
