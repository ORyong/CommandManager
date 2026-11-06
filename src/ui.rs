//! 命令管理器的界面状态机：页面栈、选择、拖拽、贴边与后台定时。
mod views;
use crate::{
    platform::{self, Desktop, Event, Singleton},
    style::{self, ThemeChoice, tint},
    worker::{Job, Update, Worker},
};
use command_manager::{
    model::{BUILTIN_GROUP, Command, Library, all_builtin_ids, normalize_tags},
    search,
    selection::Selection,
};
use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    v_flex,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DockEdge {
    Left,
    Right,
}

/// Docked window keeps its on-screen position so it can slide back out.
#[derive(Clone, Copy)]
struct Dock {
    edge: DockEdge,
    shown_left: i32,
    shown_top: i32,
    width: i32,
}

#[derive(Clone)]
enum VisibleRow {
    Folder(String),
    Command(Command),
}

#[derive(Clone)]
enum Page {
    List,
    Editor,
    Detail(Command),
    Manage,
    Text(TextOp),
    Settings,
    Confirm(Confirmation),
}
#[derive(Clone)]
enum TextOp {
    AddGroup,
    RenameGroup(String),
    AddTag,
    RenameTag(String),
}
#[derive(Clone)]
enum Confirmation {
    Commands(Vec<String>),
    Group(String),
    Tag(String),
    ClearAll,
    ClearBuiltins,
    Import(Library),
}

struct Editor {
    original: Command,
    name: Entity<InputState>,
    body: Entity<InputState>,
    notes: Entity<InputState>,
    tags: Entity<InputState>,
    groups: Vec<String>,
    _tag_submit: Subscription,
}

pub struct Manager {
    library: Library,
    worker: Worker,
    desktop: Desktop,
    singleton: Singleton,
    page: Page,
    editor: Option<Editor>,
    search: Entity<InputState>,
    aux: Entity<InputState>,
    hotkey: Entity<InputState>,
    autostart: bool,
    theme: ThemeChoice,
    pending_settings: Option<platform::Settings>,
    group: Option<String>,
    tags: Vec<String>,
    any_tag: bool,
    selection: Selection,
    selecting: bool,
    selected_only: bool,
    busy: bool,
    status: String,
    status_seen: String,
    status_deadline: Option<Instant>,
    error: bool,
    copy_deadline: Option<Instant>,
    copied: Vec<String>,
    visible: bool,
    focus_seen: bool,
    stay_open: bool,
    tags_open: bool,
    cursor: usize,
    scroll: ScrollHandle,
    dock: Option<Dock>,
    dock_revealed: bool,
    /// False right after docking, so resting the cursor on the edge does not
    /// immediately slide the panel back out. Cleared once the cursor leaves.
    dock_armed: bool,
    dock_leave: Option<Instant>,
    drag_started: Option<Instant>,
    /// 静默启动时窗口从未显示，首帧之后补一次工作集回收。
    trimmed: bool,
    _subscriptions: Vec<Subscription>,
}

impl Manager {
    pub fn new(
        directory: PathBuf,
        desktop: Desktop,
        singleton: Singleton,
        stay_open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索命令、备注或标签…"));
        let aux = cx.new(|cx| InputState::new(window, cx));
        let hotkey = cx.new(|cx| InputState::new(window, cx));
        let search_subscription =
            cx.subscribe_in(&search, window, |this: &mut Self, _, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reset_list_position();
                    this.fit_window(window, cx);
                    cx.notify();
                }
            });
        let text_submit = cx.subscribe_in(&aux, window, |this: &mut Self, _, event, _, cx| {
            if matches!(
                event,
                InputEvent::PressEnter {
                    secondary: false,
                    shift: false
                }
            ) && !this.busy
            {
                if let Page::Text(op) = this.page.clone() {
                    this.save_text(op, cx);
                }
            }
        });
        let activation = cx.observe_window_activation(window, |this, window, _| {
            if window.is_window_active() {
                this.focus_seen = true;
            }
            if this.focus_seen
                && !this.stay_open
                && !window.is_window_active()
                && matches!(this.page, Page::List)
                && !this.busy
                && this.copy_deadline.is_none()
                && this.dock.is_none()
            {
                platform::hide(window);
                this.visible = false;
            }
        });
        let worker = Worker::new(directory.join("saved_commands.json"));
        let initial_error = desktop.initial_error.clone().unwrap_or_default();
        let theme = desktop.settings.theme;
        let mut this = Self {
            library: Library::default(),
            worker,
            autostart: desktop.settings.autostart,
            theme,
            desktop,
            singleton,
            page: Page::List,
            editor: None,
            search,
            aux,
            hotkey,
            pending_settings: None,
            group: None,
            tags: vec![],
            any_tag: false,
            selection: Selection::default(),
            selecting: false,
            selected_only: false,
            busy: false,
            status: initial_error.clone(),
            status_seen: initial_error.clone(),
            status_deadline: None,
            error: !initial_error.is_empty(),
            copy_deadline: None,
            copied: vec![],
            visible: true,
            trimmed: false,
            focus_seen: false,
            stay_open,
            tags_open: false,
            cursor: 0,
            scroll: ScrollHandle::new(),
            dock: None,
            dock_revealed: true,
            dock_armed: true,
            dock_leave: None,
            drag_started: None,
            _subscriptions: vec![search_subscription, text_submit, activation],
        };
        this.queue(
            Box::new(|store| {
                Ok(Update {
                    library: Some(store.read()?),
                    import: None,
                    message: String::new(),
                    close_editor: false,
                })
            }),
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            // 16 ms is only needed while the panel is on screen; in the tray a
            // slower poll keeps the hotkey responsive without burning CPU.
            let mut delay = 16;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(delay))
                    .await;
                match this.update_in(cx, |this, window, cx| this.tick(window, cx)) {
                    Ok(visible) => delay = if visible { 16 } else { 100 },
                    Err(_) => break,
                }
            }
        })
        .detach();
        let weak = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let _ = weak.update(cx, |this, _| {
                platform::hide(window);
                this.visible = false;
            });
            false
        });
        this.search.update(cx, |input, cx| input.focus(window, cx));
        this
    }

    /// Returns whether the panel is still on screen, so the tick timer can slow
    /// down while it sits in the tray.
    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.trimmed && !platform::is_visible(window) {
            platform::release_working_set();
            self.trimmed = true;
        }
        let updates: Vec<_> = self.worker.rx.try_iter().collect();
        for update in updates {
            self.busy = false;
            if let Some(settings) = self.pending_settings.take() {
                self.desktop.finish_settings(settings, update.is_ok());
            }
            match update {
                Ok(update) => {
                    if let Some(library) = update.library {
                        self.library = library;
                        let ids: Vec<_> = self
                            .library
                            .visible_commands()
                            .iter()
                            .map(|c| c.id.clone())
                            .collect();
                        self.selection.ids.retain(|id| ids.contains(id));
                        self.tags.retain(|tag| self.library.tags.contains(tag));
                        if self.group.as_ref().is_some_and(|id| {
                            id != BUILTIN_GROUP && !self.library.groups.iter().any(|g| &g.id == id)
                        }) {
                            self.group = None;
                        }
                    }
                    if let Some(import) = update.import {
                        self.page = Page::Confirm(Confirmation::Import(import));
                    } else if update.close_editor {
                        self.page = Page::List;
                        self.editor = None;
                        self.fit_window(window, cx);
                    }
                    if !update.message.is_empty() {
                        self.status = update.message;
                        self.error = false;
                    }
                }
                Err(error) => {
                    self.status = format!("操作失败：{error:#}");
                    self.error = true;
                }
            }
            self.fit_window(window, cx);
            cx.notify();
        }
        let mut events = self.desktop.poll();
        if self.singleton.take_show() {
            events.push(Event::Show);
        }
        for event in events {
            match event {
                Event::Quit => {
                    cx.quit();
                    return false;
                }
                Event::Show => self.reveal(window, cx),
                Event::Toggle => {
                    if self.visible && platform::is_visible(window) {
                        platform::hide(window);
                        self.visible = false;
                        self.copy_deadline = None;
                    } else {
                        self.reveal(window, cx);
                    }
                }
                Event::Settings => {
                    self.reveal(window, cx);
                    self.open_settings(window, cx);
                }
            }
        }
        if self
            .copy_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.copy_deadline = None;
            self.copied.clear();
            self.selection.ids.clear();
            self.selecting = false;
            self.selected_only = false;
            platform::hide(window);
            self.visible = false;
            cx.notify();
        }
        // Toasts own their own lifetime; a new message restarts the clock.
        if self.status != self.status_seen {
            self.status_seen = self.status.clone();
            self.status_deadline = if self.status.is_empty() {
                None
            } else {
                Some(Instant::now() + Duration::from_millis(2600))
            };
        } else if self
            .status_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.status.clear();
            self.status_seen.clear();
            self.status_deadline = None;
            cx.notify();
        }
        if let Some(started) = self.drag_started {
            if !platform::left_button_down() && started.elapsed() > Duration::from_millis(120) {
                self.drag_started = None;
                self.finish_drag(window, cx);
            }
        }
        self.update_dock(window, cx);
        self.visible
    }

    fn queue(&mut self, job: Job, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        match self.worker.tx.send(job) {
            Ok(()) => {
                self.busy = true;
                self.status = "处理中…".into();
                self.error = false;
            }
            Err(_) => {
                self.status = "后台保存服务已停止，请重启程序".into();
                self.error = true;
            }
        }
        cx.notify();
    }

    /// Same as the hotkey's hide branch, triggered from the toolbar button.
    fn hide_to_tray(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        platform::hide(window);
        self.visible = false;
        self.copy_deadline = None;
        cx.notify();
    }

    fn reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copy_deadline = None;
        self.copied.clear();
        self.status.clear();
        self.status_seen.clear();
        self.status_deadline = None;
        if matches!(self.page, Page::List) {
            self.search.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.focus(window, cx);
            });
        }
        if let Some(dock) = self.dock {
            if !self.dock_revealed {
                platform::move_window(window, dock.shown_left, dock.shown_top);
                self.dock_revealed = true;
                self.dock_leave = None;
            }
        }
        platform::show(window);
        self.visible = true;
        cx.notify();
    }

    /// Starts a system window drag. The hit-test is posted, so the event
    /// handler returns immediately and the result is picked up in `tick`.
    pub(super) fn begin_window_drag(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(dock) = self.dock {
            platform::move_window(window, dock.shown_left, dock.shown_top);
            self.dock_revealed = true;
            self.dock_leave = None;
        }
        self.drag_started = Some(Instant::now());
        platform::begin_drag(window);
    }

    /// Runs after the system drag loop: snaps the window to a screen edge and
    /// leaves a thin sliver on screen, like a docked IM panel.
    fn finish_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rect = platform::window_rect(window);
        let area = platform::work_area(window);
        if area.width() <= 0 || rect.width() <= 0 {
            return;
        }
        const SNAP: i32 = 16;
        const PEEK: i32 = 6;
        let edge = if rect.left <= area.left + SNAP {
            Some(DockEdge::Left)
        } else if rect.right >= area.right - SNAP {
            Some(DockEdge::Right)
        } else {
            None
        };
        self.dock_leave = None;
        match edge {
            Some(edge) => {
                // Reveal fully on screen: a drag can leave the panel half off.
                let shown_top = rect
                    .top
                    .clamp(area.top, (area.bottom - rect.height()).max(area.top));
                let shown_left = rect.left.max(area.left);
                self.dock = Some(Dock {
                    edge,
                    shown_left,
                    shown_top,
                    width: rect.width(),
                });
                self.dock_revealed = false;
                self.dock_armed = false;
                let hidden_left = match edge {
                    DockEdge::Left => area.left - rect.width() + PEEK,
                    DockEdge::Right => area.right - PEEK,
                };
                platform::move_window(window, hidden_left, shown_top);
            }
            None => {
                self.dock = None;
                self.dock_revealed = true;
            }
        }
        cx.notify();
    }

    fn update_dock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dock) = self.dock else {
            return;
        };
        if !self.visible {
            return;
        }
        let area = platform::work_area(window);
        if area.width() <= 0 {
            return;
        }
        let rect = platform::window_rect(window);
        let (mx, my) = platform::cursor_pos();
        const PEEK: i32 = 6;
        if !self.dock_revealed {
            let near_edge = match dock.edge {
                DockEdge::Left => mx <= area.left + PEEK + 2,
                DockEdge::Right => mx >= area.right - PEEK - 2,
            };
            if !near_edge {
                self.dock_armed = true;
            }
            if near_edge && self.dock_armed {
                platform::move_window(window, dock.shown_left, dock.shown_top);
                self.dock_revealed = true;
                self.dock_leave = None;
                cx.notify();
            }
        } else if rect.contains(mx, my) {
            self.dock_leave = None;
        } else {
            let now = Instant::now();
            let deadline = *self
                .dock_leave
                .get_or_insert_with(|| now + Duration::from_millis(700));
            if now >= deadline {
                let hidden_left = match dock.edge {
                    DockEdge::Left => area.left - dock.width + PEEK,
                    DockEdge::Right => area.right - PEEK,
                };
                platform::move_window(window, hidden_left, dock.shown_top);
                self.dock_revealed = false;
                self.dock_armed = true;
                self.dock_leave = None;
                cx.notify();
            }
        }
    }

    fn reset_list_position(&mut self) {
        self.cursor = 0;
        self.scroll.set_offset(point(px(0.0), px(0.0)));
        self.scroll.scroll_to_top_of_item(0);
    }

    /// Folders and commands share one height, so the estimate is a simple sum.
    pub(super) fn command_height(_command: &Command) -> f32 {
        style::ROW_HEIGHT
    }

    /// Put the panel back in the middle of the work area and keep it inside it.
    ///
    /// `Window::resize` takes logical pixels while the Win32 rectangle and work
    /// area are physical, so the maths run in physical pixels. The panel keeps
    /// its position when the requested size already matches, which is what
    /// happens on every search keystroke — a drag stays where the user left it.
    fn place(window: &mut Window, width: f32, height: f32) {
        let area = platform::work_area(window);
        if area.width() <= 0 || area.height() <= 0 {
            window.resize(size(px(width), px(height)));
            return;
        }
        let scale = window.scale_factor().max(0.5);
        let margin = (8.0 * scale).round() as i32;
        let width = ((width * scale).round() as i32).min((area.width() - margin * 2).max(320));
        let height = ((height * scale).round() as i32).min((area.height() - margin * 2).max(240));
        let rect = platform::window_rect(window);
        let resized = rect.width() != width || rect.height() != height;
        let (left, top) = if resized {
            (
                area.left + (area.width() - width) / 2,
                area.top + (area.height() - height) / 2,
            )
        } else {
            (rect.left, rect.top)
        };
        // Centring is only safe after the size is clamped, and a drag may have
        // left the panel hanging over the taskbar.
        let left = left.clamp(area.left, (area.right - margin - width).max(area.left));
        let top = top.clamp(area.top, (area.bottom - margin - height).max(area.top));
        window.resize(size(px(width as f32 / scale), px(height as f32 / scale)));
        if left != rect.left || top != rect.top {
            platform::move_window(window, left, top);
        }
    }

    /// Every page is one fixed rectangle: no more growing with the list.
    fn fit_window(&self, window: &mut Window, _cx: &App) {
        Self::place(window, style::PANEL_WIDTH, style::PANEL_HEIGHT);
    }

    fn rows(&self, cx: &App) -> Vec<VisibleRow> {
        let query = self.search.read(cx).value();
        let folders = query.trim().is_empty()
            && self.group.is_none()
            && self.tags.is_empty()
            && !self.selected_only;
        let mut rows = Vec::new();
        if folders {
            if self
                .library
                .visible_commands()
                .iter()
                .any(|c| c.groups.contains(&BUILTIN_GROUP.to_owned()))
            {
                rows.push(VisibleRow::Folder(BUILTIN_GROUP.into()));
            }
            rows.extend(
                self.library
                    .groups
                    .iter()
                    .map(|g| VisibleRow::Folder(g.id.clone())),
            );
        }
        rows.extend(self.commands(cx).into_iter().map(VisibleRow::Command));
        rows
    }

    fn commands(&self, cx: &App) -> Vec<Command> {
        if self.selected_only {
            let mut commands = self.library.visible_commands();
            commands.retain(|c| self.selection.ids.contains(&c.id));
            commands.sort_by_key(|c| self.selection.ordinal(&c.id));
            return commands;
        }
        let query = self.search.read(cx).value().to_string();
        let mut commands = search::filter(
            &self.library.visible_commands(),
            query.trim(),
            &self.tags,
            self.any_tag,
        );
        if self.selected_only {
            commands.retain(|c| self.selection.ids.contains(&c.id));
            commands.sort_by_key(|c| self.selection.ordinal(&c.id));
        } else if query.trim().is_empty() {
            if let Some(group) = &self.group {
                commands.retain(|c| c.groups.contains(group));
            } else if self.tags.is_empty() {
                commands.retain(|c| c.groups.is_empty());
            }
        }
        commands
    }

    fn copy(&mut self, ids: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let selection = Selection { ids: ids.clone() };
        let text = if ids.len() == 1 {
            self.library
                .visible_commands()
                .into_iter()
                .find(|c| c.id == ids[0])
                .map(|c| c.command)
                .ok_or_else(|| anyhow::anyhow!("命令已不存在"))
        } else {
            selection.text(&self.library.visible_commands())
        };
        match text {
            Ok(text) => {
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                if cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref()
                    != Some(text.as_str())
                {
                    self.status = "复制失败：剪贴板校验未通过，请重试".into();
                    self.error = true;
                    self.copy_deadline = None;
                } else {
                    self.copied = ids;
                    self.status = "已复制".into();
                    self.error = false;
                    self.copy_deadline = Some(Instant::now() + Duration::from_millis(800));
                }
            }
            Err(error) => {
                self.status = error.to_string();
                self.error = true;
            }
        }
        let _ = window;
        cx.notify();
    }

    fn activate(&mut self, command: &Command, window: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            self.selection.toggle(&command.id);
            cx.notify();
        } else {
            self.copy(vec![command.id.clone()], window, cx);
        }
    }

    fn edit(
        &mut self,
        mut command: Command,
        duplicate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if duplicate {
            command.id = String::new();
            command.name = format!("{} 副本", command.name);
        }
        let name = cx.new(|cx| InputState::new(window, cx));
        let body = cx.new(|cx| InputState::new(window, cx).multi_line(true).soft_wrap(true));
        let notes = cx.new(|cx| InputState::new(window, cx).multi_line(true).soft_wrap(true));
        let tags =
            cx.new(|cx| InputState::new(window, cx).placeholder("用逗号分隔，例如 Docker, 开发"));
        name.update(cx, |input, cx| {
            input.set_value(command.name.clone(), window, cx)
        });
        body.update(cx, |input, cx| {
            input.set_value(command.command.clone(), window, cx)
        });
        notes.update(cx, |input, cx| {
            input.set_value(command.notes.clone(), window, cx)
        });
        tags.update(cx, |input, cx| {
            input.set_value(command.tags.join(", "), window, cx)
        });
        let tag_submit = cx.subscribe_in(&tags, window, |this: &mut Self, _, event, _, cx| {
            if matches!(
                event,
                InputEvent::PressEnter {
                    secondary: false,
                    shift: false
                }
            ) && matches!(this.page, Page::Editor)
                && !this.busy
            {
                this.save_editor(cx);
            }
        });
        self.editor = Some(Editor {
            groups: command.groups.clone(),
            original: command,
            name,
            body,
            notes,
            tags,
            _tag_submit: tag_submit,
        });
        self.page = Page::Editor;
        self.status.clear();
        self.copy_deadline = None;
        Self::place(window, style::PANEL_WIDTH, style::PANEL_HEIGHT);
        cx.notify();
    }

    fn save_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let builtin = editor.original.builtin();
        let mut command = editor.original.clone();
        if !builtin {
            command.name = editor.name.read(cx).value().to_string();
            command.command = editor.body.read(cx).value().to_string();
        }
        command.notes = editor.notes.read(cx).value().to_string();
        command.tags = editor
            .tags
            .read(cx)
            .value()
            .split([',', '，', ';', '；'])
            .map(str::to_owned)
            .collect();
        command.groups = editor.groups.clone();
        if let Err(error) = command.normalize() {
            self.status = error.to_string();
            self.error = true;
            cx.notify();
            return;
        }
        self.queue(
            Box::new(move |store| {
                Ok(Update::saved(
                    store.mutate(|library| library.save(command))?,
                    "已保存",
                ))
            }),
            cx,
        );
    }

    fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Leaving settings without saving drops the previewed skin.
        if matches!(self.page, Page::Settings) && self.theme != self.desktop.settings.theme {
            self.theme = self.desktop.settings.theme;
            style::apply(self.theme, cx);
        }
        self.page = Page::List;
        self.editor = None;
        self.status.clear();
        self.fit_window(window, cx);
        self.search.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn text_page(
        &mut self,
        op: TextOp,
        value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.aux.update(cx, |input, cx| {
            input.set_value(value, window, cx);
            input.focus(window, cx);
        });
        self.page = Page::Text(op);
        self.status.clear();
        self.status_deadline = None;
        Self::place(window, style::PANEL_WIDTH, 300.0);
        cx.notify();
    }

    fn save_text(&mut self, op: TextOp, cx: &mut Context<Self>) {
        let value = self.aux.read(cx).value().to_string();
        self.queue(
            Box::new(move |store| {
                Ok(Update::saved(
                    store.mutate(|library| {
                        match op {
                            TextOp::AddGroup => library.add_group(&value)?,
                            TextOp::RenameGroup(id) => library.rename_group(&id, &value)?,
                            TextOp::AddTag => {
                                let tags = normalize_tags(&[value])?;
                                anyhow::ensure!(!tags.is_empty(), "标签不能为空");
                                library.tags.extend(tags);
                            }
                            TextOp::RenameTag(old) => library.rename_tag(&old, &value)?,
                        }
                        Ok(())
                    })?,
                    "已保存分类",
                ))
            }),
            cx,
        );
    }

    fn confirm(&mut self, action: Confirmation, cx: &mut Context<Self>) {
        self.page = Page::Confirm(action);
        self.copy_deadline = None;
        if let Some(handle) = cx.active_window() {
            let _ = handle.update(cx, |_, window, _| {
                Self::place(window, style::PANEL_WIDTH, 340.0);
            });
        }
        cx.notify();
    }

    fn apply_confirmation(&mut self, action: Confirmation, cx: &mut Context<Self>) {
        self.queue(
            Box::new(move |store| {
                Ok(Update::saved(
                    store.mutate(|library| {
                        match action {
                            Confirmation::Commands(ids) => library.remove(&ids),
                            Confirmation::Group(id) => library.remove_group(&id)?,
                            Confirmation::Tag(tag) => library.remove_tag(&tag),
                            Confirmation::ClearAll => {
                                let mut ids: Vec<_> = all_builtin_ids().into_iter().collect();
                                ids.extend(library.commands.iter().map(|c| c.id.clone()));
                                library.remove(&ids);
                            }
                            Confirmation::ClearBuiltins => {
                                library.remove(&all_builtin_ids().into_iter().collect::<Vec<_>>())
                            }
                            Confirmation::Import(replacement) => *library = replacement,
                        }
                        Ok(())
                    })?,
                    "操作已完成",
                ))
            }),
            cx,
        );
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.desktop.settings.hotkey.clone();
        self.hotkey
            .update(cx, |input, cx| input.set_value(value, window, cx));
        self.autostart = self.desktop.settings.autostart;
        self.page = Page::Settings;
        self.copy_deadline = None;
        Self::place(window, style::PANEL_WIDTH, style::PANEL_HEIGHT);
        cx.notify();
    }

    fn save_settings(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let settings = platform::Settings {
            hotkey: self.hotkey.read(cx).value().to_string(),
            autostart: self.autostart,
            theme: self.theme,
        };
        if let Err(error) = self.desktop.prepare_settings(&settings) {
            self.status = format!("{error:#}");
            self.error = true;
            cx.notify();
            return;
        }
        let path = self.desktop.settings_path();
        let old_autostart = self.desktop.settings.autostart;
        let next = settings.clone();
        self.queue(
            Box::new(move |_| {
                let result = (|| {
                    platform::autostart(next.autostart)?;
                    command_manager::store::write_json(&path, &next)?;
                    Ok(())
                })();
                if let Err(error) = result {
                    let _ = platform::autostart(old_autostart);
                    return Err(error);
                }
                Ok(Update {
                    library: None,
                    import: None,
                    message: "设置已保存".into(),
                    close_editor: false,
                })
            }),
            cx,
        );
        if self.busy {
            self.pending_settings = Some(settings);
        } else {
            self.desktop.finish_settings(settings, false);
        }
    }

    fn import(&mut self, cx: &mut Context<Self>) {
        self.queue(
            Box::new(|_| {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("命令库 JSON", &["json"])
                    .pick_file()
                else {
                    return Ok(Update {
                        library: None,
                        import: None,
                        message: "已取消导入".into(),
                        close_editor: false,
                    });
                };
                anyhow::ensure!(
                    std::fs::metadata(&path)?.len() <= 64 * 1024 * 1024,
                    "文件超过 64 MB"
                );
                let library = Library::decode(&std::fs::read(path)?)?;
                Ok(Update {
                    library: None,
                    import: Some(library),
                    message: String::new(),
                    close_editor: false,
                })
            }),
            cx,
        );
    }

    fn export(&mut self, cx: &mut Context<Self>) {
        self.queue(
            Box::new(|store| {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("命令库 JSON", &["json"])
                    .set_file_name("saved_commands.json")
                    .save_file()
                else {
                    return Ok(Update {
                        library: None,
                        import: None,
                        message: "已取消导出".into(),
                        close_editor: false,
                    });
                };
                store.export(&path)?;
                Ok(Update::saved(store.read()?, "已导出命令、分组、备注和标签"))
            }),
            cx,
        );
    }
}

impl Render for Manager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let skin = *style::palette(cx);
        let content = match self.page.clone() {
            Page::List => self.render_list(window, cx),
            Page::Editor => self.render_editor(window, cx),
            Page::Detail(command) => self.render_detail(command, window, cx),
            Page::Manage => self.render_manage(window, cx),
            Page::Text(op) => self.render_text(op, window, cx),
            Page::Settings => self.render_settings(window, cx),
            Page::Confirm(action) => self.render_confirmation(action, window, cx),
        };
        let panel = v_flex()
            .relative()
            .size_full()
            .bg(tint(skin.panel))
            .overflow_hidden()
            .child(content)
            .children(self.render_toast(cx));
        v_flex()
            .id("command-manager")
            .size_full()
            .bg(tint(skin.panel))
            .text_color(tint(skin.fg))
            .text_sm()
            .overflow_hidden()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if !matches!(this.page, Page::List) {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "escape" => {
                        platform::hide(window);
                        this.visible = false;
                    }
                    "up" => {
                        this.cursor = this.cursor.saturating_sub(1);
                        this.scroll.scroll_to_item(this.cursor);
                        cx.notify();
                    }
                    "down" => {
                        this.cursor = (this.cursor + 1).min(this.rows(cx).len().saturating_sub(1));
                        this.scroll.scroll_to_item(this.cursor);
                        cx.notify();
                    }
                    "enter" => match this.rows(cx).get(this.cursor).cloned() {
                        Some(VisibleRow::Folder(id)) => {
                            this.group = Some(id);
                            this.reset_list_position();
                            this.fit_window(window, cx);
                            cx.notify();
                        }
                        Some(VisibleRow::Command(command)) => this.activate(&command, window, cx),
                        None => {}
                    },
                    _ => {}
                }
            }))
            .child(panel)
    }
}
