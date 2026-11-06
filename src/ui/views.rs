use super::*;
use crate::style::Palette;

impl Manager {
    // ------------------------------------------------------------- shared bits

    fn leading(
        &self,
        icon: IconName,
        tone: Hsla,
        skin: &Palette,
        filled: bool,
    ) -> AnyElement {
        div()
            .size(px(28.0))
            .flex_shrink_0()
            .rounded(px(8.0))
            .when(filled, |slot| slot.bg(tint(skin.raised)))
            .flex()
            .items_center()
            .justify_center()
            .child(Icon::new(icon).size(px(15.0)).text_color(tone))
            .into_any_element()
    }

    fn tag_chips(tags: &[String], limit: usize, skin: &Palette) -> AnyElement {
        let mut row = h_flex().flex_shrink_0().gap(px(4.0)).overflow_hidden();
        for tag in tags.iter().take(limit) {
            row = row.child(
                div()
                    .px(px(6.0))
                    .rounded(px(style::RADIUS_CHIP))
                    .bg(tint(skin.tag_bg))
                    .text_size(px(10.5))
                    .text_color(tint(skin.tag_fg))
                    .child(tag.clone()),
            );
        }
        if tags.len() > limit {
            row = row.child(
                div()
                    .text_size(px(10.5))
                    .text_color(tint(skin.fg_subtle))
                    .child(format!("+{}", tags.len() - limit)),
            );
        }
        row.into_any_element()
    }

    fn field_label(text: &str, skin: &Palette) -> AnyElement {
        div()
            .text_size(px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(tint(skin.fg_subtle))
            .child(text.to_owned())
            .into_any_element()
    }

    pub(super) fn render_toast(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.status.is_empty() {
            return None;
        }
        let skin = *style::palette(cx);
        let tone = if self.error { skin.danger } else { skin.success };
        let icon = if self.error {
            IconName::CircleX
        } else {
            IconName::CircleCheck
        };
        Some(
            div()
                .absolute()
                .bottom(px(style::FOOTER_HEIGHT + 8.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    h_flex()
                        .max_w(px(430.0))
                        .px(px(10.0))
                        .py(px(5.0))
                        .gap(px(6.0))
                        .rounded_full()
                        .bg(tint(skin.raised))
                        .border_1()
                        .border_color(tint(skin.border_strong))
                        .shadow(vec![BoxShadow {
                            color: tint(skin.shadow),
                            offset: point(px(0.0), px(2.0)),
                            blur_radius: px(10.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                        .child(Icon::new(icon).size(px(13.0)).text_color(tint(tone)))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.0))
                                .text_color(tint(skin.fg))
                                .child(self.status.clone()),
                        ),
                )
                .into_any_element(),
        )
    }

    // ------------------------------------------------------------------- list

    pub(super) fn render_list(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let skin = *style::palette(cx);
        let query = self.search.read(cx).value().to_string();
        let commands = self.commands(cx);
        let folders = query.trim().is_empty()
            && self.group.is_none()
            && self.tags.is_empty()
            && !self.selected_only;

        let mut list = v_flex()
            .id("command-list")
            .w_full()
            .gap(px(style::ROW_GAP))
            .p(px(style::LIST_PAD));

        // ---- folder rows: one elevated card, the commands stay flat below
        let mut card_rows: Vec<AnyElement> = Vec::new();
        if folders {
            for (index, (id, name)) in
                std::iter::once((BUILTIN_GROUP.to_owned(), "内置命令".to_owned()))
                    .chain(
                        self.library
                            .groups
                            .iter()
                            .map(|g| (g.id.clone(), g.name.clone())),
                    )
                    .enumerate()
            {
                let count = self
                    .library
                    .visible_commands()
                    .iter()
                    .filter(|c| c.groups.contains(&id))
                    .count();
                if id == BUILTIN_GROUP && count == 0 {
                    continue;
                }
                let enter = id.clone();
                let delete = id.clone();
                let hover_group = SharedString::from(format!("folder-row-hover-{id}"));
                let is_cursor = index == self.cursor;
                let divided = !card_rows.is_empty();
                card_rows.push(
                    h_flex()
                        .id(SharedString::from(format!("folder-{id}")))
                        .group(hover_group.clone())
                        .w_full()
                        .h(px(style::FOLDER_HEIGHT))
                        .flex_shrink_0()
                        .items_center()
                        .gap(px(10.0))
                        .px(px(10.0))
                        .when(divided, |row| {
                            row.border_t_1().border_color(tint(skin.border))
                        })
                        .cursor_pointer()
                        .role(Role::Button)
                        .aria_label(name.clone())
                        .when(is_cursor, |row| row.bg(tint(skin.hover)))
                        .when(!is_cursor, |row| row.hover(move |s| s.bg(tint(skin.hover))))
                        .active(move |s| s.bg(tint(skin.selected)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.group = Some(enter.clone());
                            this.reset_list_position();
                            this.fit_window(window, cx);
                            cx.notify();
                        }))
                        .child(self.leading(
                            IconName::Folder,
                            tint(skin.accent),
                            &skin,
                            false,
                        ))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(1.0))
                                .child(
                                    div()
                                        .truncate()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(tint(skin.fg))
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(tint(skin.fg_subtle))
                                        .child(format!("{count} 条命令")),
                                ),
                        )
                        .child(
                            div()
                                .opacity(0.0)
                                .group_hover(hover_group, |slot| slot.opacity(1.0))
                                .child(
                                    Button::new(SharedString::from(format!("delete-group-{id}")))
                                        .icon(IconName::Close)
                                        .ghost()
                                        .small()
                                        .tooltip("删除分组，保留命令")
                                        .aria_label("删除分组，保留命令")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            this.confirm(
                                                if delete == BUILTIN_GROUP {
                                                    Confirmation::ClearBuiltins
                                                } else {
                                                    Confirmation::Group(delete.clone())
                                                },
                                                cx,
                                            );
                                        })),
                                ),
                        )
                        .child(
                            Icon::new(IconName::ChevronRight)
                                .size(px(14.0))
                                .text_color(tint(skin.fg_subtle)),
                        )
                        .into_any_element(),
                );
            }
        }

        if !card_rows.is_empty() {
            let mut card = v_flex()
                .w_full()
                .flex_shrink_0()
                .rounded(px(style::RADIUS_CONTROL + 3.0))
                .bg(tint(skin.raised))
                .border_1()
                .border_color(tint(skin.border))
                .overflow_hidden()
                // A wider break between the folder block and the command list.
                .when(folders && !commands.is_empty(), |card| {
                    card.mb(px(style::BLOCK_GAP))
                });
            for row in card_rows {
                card = card.child(row);
            }
            list = list.child(card);
        }

        // ---- command rows
        let offset = if folders {
            self.rows(cx).len() - commands.len()
        } else {
            0
        };
        for (index, command) in commands.iter().enumerate() {
            list = list.child(self.render_row(offset + index, command, window, cx));
        }

        // ---- empty states
        if commands.is_empty() && !folders {
            list = list.child(self.render_empty(&query, cx));
        } else if commands.is_empty() && folders && self.library.groups.is_empty() {
            list = list.child(self.render_empty("", cx));
        }

        // ---- tag filter panel
        let mut tags = h_flex()
            .id("tag-filter-list")
            .w_full()
            .max_h(px(style::TAG_CHIPS_MAX_HEIGHT))
            .overflow_y_scroll()
            .flex_wrap()
            .gap(px(4.0));
        for tag in &self.library.tags {
            let value = tag.clone();
            tags = tags.child(
                Button::new(SharedString::from(format!("filter-tag-{tag}")))
                    .label(tag.clone())
                    .small()
                    .ghost()
                    .selected(self.tags.contains(tag))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if this.tags.contains(&value) {
                            this.tags.retain(|t| t != &value);
                        } else {
                            this.tags.push(value.clone());
                        }
                        this.reset_list_position();
                        this.fit_window(window, cx);
                        cx.notify();
                    })),
            );
        }

        // ---- search bar
        let search_box = h_flex()
            .w_full()
            .h(px(style::SEARCH_BAR_HEIGHT))
            .flex_shrink_0()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(tint(skin.border_strong))
            .bg(tint(skin.panel))
            .overflow_hidden()
            .child(
                Icon::new(IconName::Search)
                    .size(px(14.0))
                    .text_color(tint(skin.fg_subtle)),
            )
            .child(
                Input::new(&self.search)
                    .flex_1()
                    .appearance(false)
                    .focus_bordered(false)
                    .cleanable(true)
                    .text_size(px(13.0)),
            )
            .when(!folders, |bar| {
                bar.child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(11.0))
                        .text_color(tint(skin.fg_subtle))
                        .child(format!("{} 条", commands.len())),
                )
            });

        // ---- toolbar
        let mut toolbar = h_flex()
            .w_full()
            .h(px(style::TOOLBAR_HEIGHT))
            .px(px(10.0))
            .gap(px(2.0))
            .items_center()
            .flex_shrink_0();

        if self.group.is_some() {
            toolbar = toolbar.child(
                Button::new("back-groups")
                    .icon(IconName::ChevronLeft)
                    .label("全部命令")
                    .ghost()
                    .small()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.group = None;
                        this.reset_list_position();
                        this.fit_window(window, cx);
                        cx.notify();
                    })),
            );
        }

        toolbar = toolbar.child(
            Button::new("select-mode")
                .label(if self.selecting { "取消" } else { "多选" })
                .small()
                .ghost()
                .selected(self.selecting)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.selecting = !this.selecting;
                    if !this.selecting {
                        this.selection.ids.clear();
                        this.selected_only = false;
                        this.reset_list_position();
                    }
                    this.fit_window(window, cx);
                    cx.notify();
                })),
        );

        if self.selecting {
            toolbar = toolbar
                .child(
                    Button::new("copy-selected")
                        .label(format!("复制 {} 条", self.selection.ids.len()))
                        .small()
                        .ghost()
                        .disabled(self.selection.ids.is_empty())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.copy(this.selection.ids.clone(), window, cx)
                        })),
                )
                .child(
                    Button::new("view-selected")
                        .label("已选")
                        .small()
                        .ghost()
                        .selected(self.selected_only)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.selected_only = !this.selected_only;
                            this.reset_list_position();
                            this.fit_window(window, cx);
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("clear-selected")
                        .label("清空")
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.selection.ids.clear();
                            if this.selected_only {
                                this.reset_list_position();
                            }
                            this.fit_window(window, cx);
                            cx.notify();
                        })),
                );
        }

        toolbar = toolbar.child(
            div()
                .flex_1()
                .h_full()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, window, cx| {
                        this.begin_window_drag(window, cx);
                    }),
                ),
        );

        if !self.library.tags.is_empty() && !self.selecting {
            toolbar = toolbar.child(
                Button::new("filter-tags")
                    .label(if self.tags.is_empty() {
                        "标签".into()
                    } else {
                        format!("标签 {}", self.tags.len())
                    })
                    .small()
                    .ghost()
                    .selected(self.tags_open)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.tags_open = !this.tags_open;
                        this.fit_window(window, cx);
                        cx.notify();
                    })),
            );
        }

        toolbar = toolbar.child(self.manage_button(cx)).child(self.hide_button(cx));

        // ---- footer
        let footer = h_flex()
            .w_full()
            .h(px(style::FOOTER_HEIGHT))
            .flex_shrink_0()
            .items_center()
            .gap(px(4.0))
            .px(px(10.0))
            .bg(tint(skin.surface))
            .border_t_1()
            .border_color(tint(skin.border))
            .child(
                Button::new("add-command")
                    .icon(IconName::Plus)
                    .label("新增命令")
                    .small()
                    .ghost()
                    .text_color(tint(skin.fg_muted))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.edit(
                            Command {
                                groups: this.group.clone().into_iter().collect(),
                                ..Default::default()
                            },
                            false,
                            window,
                            cx,
                        )
                    })),
            )
            .child(div().flex_1())
            .when(self.group.is_none(), |bar| {
                bar.child(
                    Button::new("add-group")
                        .icon(IconName::Folder)
                        .label("新建分组")
                        .small()
                        .ghost()
                        .text_color(tint(skin.fg_muted))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.text_page(TextOp::AddGroup, String::new(), window, cx)
                        })),
                )
            });

        v_flex()
            .size_full()
            .bg(tint(skin.panel))
            .child(
                // Search block and toolbar share one raised band.
                v_flex()
                    .w_full()
                    .flex_shrink_0()
                    .bg(tint(skin.surface))
                    .border_b_1()
                    .border_color(tint(skin.border))
                    .child(
                        // Empty padding of the search block doubles as a drag handle.
                        div()
                            .w_full()
                            .flex_shrink_0()
                            .px(px(10.0))
                            .pt(px(style::SEARCH_BLOCK_PAD_TOP))
                            .pb(px(style::SEARCH_BLOCK_PAD_BOTTOM))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                                    this.begin_window_drag(window, cx);
                                }),
                            )
                            .child(search_box),
                    )
                    .child(toolbar),
            )
            .when(self.tags_open, |root| {
                root.child(
                    v_flex()
                        .h(px(style::TAGS_PANEL_HEIGHT))
                        .px(px(10.0))
                        .py(px(8.0))
                        .gap(px(6.0))
                        .bg(tint(skin.surface))
                        .border_b_1()
                        .border_color(tint(skin.border))
                        .child(tags)
                        .child(
                            h_flex()
                                .gap(px(2.0))
                                .child(
                                    Button::new("tag-mode")
                                        .label(if self.any_tag {
                                            "任意标签"
                                        } else {
                                            "全部标签"
                                        })
                                        .small()
                                        .ghost()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.any_tag = !this.any_tag;
                                            this.reset_list_position();
                                            this.fit_window(window, cx);
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("clear-tags")
                                        .label("清除筛选")
                                        .small()
                                        .ghost()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.tags.clear();
                                            this.reset_list_position();
                                            this.fit_window(window, cx);
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
            })
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(command_manager::list_view::scroll_viewport(
                        list,
                        &self.scroll,
                    )),
            )
            .child(footer)
            .into_any_element()
    }

    fn render_empty(&self, query: &str, cx: &mut Context<Self>) -> AnyElement {
        let skin = *style::palette(cx);
        let searching = !query.trim().is_empty() || self.group.is_some() || !self.tags.is_empty();
        let (icon, title, hint) = if searching {
            (
                IconName::Search,
                format!("没有找到「{}」", query.trim()),
                "换个关键词，或清除筛选条件",
            )
        } else {
            (
                IconName::Inbox,
                "命令库还是空的".to_string(),
                "新增第一条命令，或从管理里导入",
            )
        };
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .py(px(28.0))
            .child(
                Icon::new(icon)
                    .size(px(28.0))
                    .text_color(tint(skin.fg_subtle)),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(tint(skin.fg_muted))
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(tint(skin.fg_subtle))
                    .child(hint),
            )
            .when(!searching, |empty| {
                empty.child(
                    div().pt(px(6.0)).child(
                        Button::new("empty-add")
                            .icon(IconName::Plus)
                            .label("新增命令")
                            .small()
                            .primary()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.edit(Command::default(), false, window, cx)
                            })),
                    ),
                )
            })
            .when(searching, |empty| {
                empty.child(
                    div().pt(px(6.0)).child(
                        Button::new("empty-clear")
                            .label("清除筛选")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.search
                                    .update(cx, |input, cx| input.set_value("", window, cx));
                                this.group = None;
                                this.tags.clear();
                                this.reset_list_position();
                                this.fit_window(window, cx);
                                cx.notify();
                            })),
                    ),
                )
            })
            .into_any_element()
    }

    /// Click-to-tray: the same path as the global hotkey and the window close
    /// button, reachable without leaving the mouse.
    fn hide_button(&self, cx: &mut Context<Self>) -> AnyElement {
        Button::new("hide-to-tray")
            .icon(IconName::Close)
            .ghost()
            .small()
            .tooltip("隐藏到托盘")
            .aria_label("隐藏到托盘")
            .on_click(cx.listener(|this, _, window, cx| this.hide_to_tray(window, cx)))
            .into_any_element()
    }

    fn manage_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let owner = cx.entity().downgrade();
        let selecting = self.selecting;
        Button::new("manage")
            .label("管理")
            .small()
            .ghost()
            .dropdown_caret(true)
            .disabled(self.busy)
            .dropdown_menu(move |menu, _, _| {
                let manage = owner.clone();
                let import = owner.clone();
                let export = owner.clone();
                let settings = owner.clone();
                menu.item(
                    PopupMenuItem::new("命令与分类管理").on_click(move |_, window, cx| {
                        let _ = manage.update(cx, |this, cx| {
                            this.page = Page::Manage;
                            this.copy_deadline = None;
                            Manager::place(window, style::PANEL_WIDTH, style::PANEL_HEIGHT);
                            cx.notify();
                        });
                    }),
                )
                .item(PopupMenuItem::new("导入命令库…").on_click(move |_, _, cx| {
                    let _ = import.update(cx, |this, cx| this.import(cx));
                }))
                .item(PopupMenuItem::new("导出命令库…").on_click(move |_, _, cx| {
                    let _ = export.update(cx, |this, cx| this.export(cx));
                }))
                .when(selecting, |menu| {
                    let select = owner.clone();
                    let delete = owner.clone();
                    menu.item(
                        PopupMenuItem::new("选择当前列表").on_click(move |_, _, cx| {
                            let _ = select.update(cx, |this, cx| {
                                for command in this.commands(cx) {
                                    if !this.selection.ids.contains(&command.id) {
                                        this.selection.ids.push(command.id);
                                    }
                                }
                                cx.notify();
                            });
                        }),
                    )
                    .item(
                        PopupMenuItem::new("删除所选命令…").on_click(move |_, _, cx| {
                            let _ = delete.update(cx, |this, cx| {
                                this.confirm(Confirmation::Commands(this.selection.ids.clone()), cx)
                            });
                        }),
                    )
                })
                .item(PopupMenuItem::new("设置").on_click(move |_, window, cx| {
                    let _ = settings.update(cx, |this, cx| this.open_settings(window, cx));
                }))
            })
            .into_any_element()
    }

    fn render_row(
        &self,
        index: usize,
        command: &Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skin = *style::palette(cx);
        let command = command.clone();
        let activate = command.clone();
        let detail = command.clone();
        let edit = command.clone();
        let delete = command.id.clone();
        let selected = self.selection.ordinal(&command.id);
        let copied = self.copied.contains(&command.id);
        let is_cursor = index == self.cursor;
        let hover_group = SharedString::from(format!("command-row-hover-{index}"));
        let actions_focus = window
            .use_keyed_state(
                SharedString::from(format!("command-actions-focus-{index}")),
                cx,
                |_, cx| cx.focus_handle().tab_stop(false),
            )
            .read(cx)
            .clone();
        let actions_focused = actions_focus.contains_focused(window, cx);

        let (row_bg, bar) = if copied {
            (tint(skin.copied), skin.success)
        } else if selected.is_some() || is_cursor {
            (tint(skin.selected), skin.accent)
        } else {
            (tint(0x00000000), skin.accent)
        };
        let neutral = copied == false && selected.is_none() && !is_cursor;

        h_flex()
            .id(SharedString::from(format!("command-{}", command.id)))
            .group(hover_group.clone())
            .relative()
            .role(Role::Button)
            .aria_label(command.name.clone())
            .w_full()
            .min_h(px(Self::command_height(&command)))
            .flex_shrink_0()
            .items_center()
            .px(px(10.0))
            .py(px(10.0))
            .gap(px(10.0))
            .rounded(px(8.0))
            .cursor_pointer()
            .bg(row_bg)
            .when(neutral, move |row| row.hover(move |s| s.bg(tint(skin.hover))))
            .on_click(cx.listener(move |this, _, window, cx| this.activate(&activate, window, cx)))
            .when(copied || selected.is_some() || is_cursor, |row| {
                row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(8.0))
                        .bottom(px(8.0))
                        .w(px(2.5))
                        .rounded_full()
                        .bg(tint(bar)),
                )
            })
            .child(if self.selecting {
                let marked = selected.is_some();
                div()
                    .size(px(20.0))
                    .flex_shrink_0()
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(marked, |badge| badge.bg(tint(skin.accent)))
                    .when(!marked, |badge| {
                        badge.border_1().border_color(tint(skin.border_strong))
                    })
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(tint(if marked {
                                skin.accent_fg
                            } else {
                                skin.fg_subtle
                            }))
                            .child(selected.map(|n| n.to_string()).unwrap_or_default()),
                    )
                    .into_any_element()
            } else {
                self.leading(IconName::SquareTerminal, tint(skin.fg_subtle), &skin, false)
            })
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(3.0))
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .gap(px(6.0))
                            .items_center()
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(tint(skin.fg))
                                    .child(command.name.clone()),
                            )
                            .when(!command.tags.is_empty(), |line| {
                                line.child(Self::tag_chips(&command.tags, 2, &skin))
                            })
                            .child(div().flex_1().min_w_0()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.0))
                            .font_family("Consolas")
                            .text_color(tint(skin.fg_muted))
                            .child(command.command.lines().collect::<Vec<_>>().join(" ")),
                    ),
            )
            .when(copied, |row| {
                row.child(
                    div()
                        .flex_shrink_0()
                        .px(px(6.0))
                        .rounded(px(style::RADIUS_CHIP))
                        .bg(tint(skin.copied))
                        .text_size(px(10.5))
                        .text_color(tint(skin.success))
                        .child("已复制"),
                )
            })
            .child(
                h_flex()
                    .id(SharedString::from(format!("row-actions-{index}")))
                    .track_focus(&actions_focus)
                    .flex_shrink_0()
                    .gap(px(2.0))
                    .opacity(if actions_focused { 1.0 } else { 0.0 })
                    .group_hover(hover_group, |row| row.opacity(1.0))
                    .child(
                        Button::new(SharedString::from(format!("detail-{}", command.id)))
                            .icon(IconName::ChevronRight)
                            .ghost()
                            .small()
                            .tooltip("完整详情")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.page = Page::Detail(detail.clone());
                                this.copy_deadline = None;
                                Manager::place(window, style::PANEL_WIDTH, style::PANEL_HEIGHT);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("edit-{}", command.id)))
                            .icon(Icon::empty().path("icons/pencil.svg"))
                            .ghost()
                            .small()
                            .tooltip("编辑命令、备注和标签")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.edit(edit.clone(), false, window, cx);
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("delete-{}", command.id)))
                            .icon(Icon::empty().path("icons/trash-2.svg"))
                            .ghost()
                            .small()
                            .tooltip("删除命令")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.confirm(Confirmation::Commands(vec![delete.clone()]), cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    // ---------------------------------------------------------------- shared UI

    fn header(&self, title: &str, skin: &Palette, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .h(px(52.0))
            .flex_shrink_0()
            .px(px(10.0))
            .gap(px(6.0))
            .items_center()
            .border_b_1()
            .border_color(tint(skin.border))
            .child(
                Button::new("back-list")
                    .icon(IconName::ChevronLeft)
                    .ghost()
                    .small()
                    .tooltip("返回")
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, window, cx| this.back(window, cx))),
            )
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tint(skin.fg))
                    .child(title.to_owned()),
            )
            .into_any_element()
    }

    fn action_bar(skin: &Palette) -> Div {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(12.0))
            .border_t_1()
            .border_color(tint(skin.border))
    }

    // ---------------------------------------------------------------- editor

    pub(super) fn render_editor(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skin = *style::palette(cx);
        let editor = self.editor.as_ref().expect("editor page owns editor");
        let builtin = editor.original.builtin();
        let mut groups = h_flex().flex_wrap().gap(px(4.0));
        let mut group_count = 0usize;
        for (id, name) in std::iter::once((BUILTIN_GROUP.to_owned(), "内置命令".to_owned()))
            .chain(
                self.library
                    .groups
                    .iter()
                    .map(|g| (g.id.clone(), g.name.clone())),
            )
        {
            group_count += 1;
            let selected = editor.groups.contains(&id);
            groups = groups.child(
                Button::new(SharedString::from(format!("edit-group-{id}")))
                    .label(name)
                    .small()
                    .ghost()
                    .selected(selected)
                    .on_click(cx.listener(move |this, checked, _, cx| {
                        if let Some(editor) = &mut this.editor {
                            if editor.groups.contains(&id) {
                                editor.groups.retain(|g| g != &id);
                            } else {
                                editor.groups.push(id.clone());
                            }
                        }
                        let _ = checked;
                        cx.notify();
                    })),
            );
        }
        if group_count == 0 {
            groups = groups.child(
                div()
                    .text_size(px(11.5))
                    .text_color(tint(skin.fg_subtle))
                    .child("暂无分组"),
            );
        }
        v_flex()
            .size_full()
            .child(self.header(
                if builtin {
                    "编辑内置命令元数据"
                } else {
                    "编辑命令"
                },
                &skin,
                cx,
            ))
            .child(
                v_flex()
                    .id("editor-fields")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(16.0))
                    .py(px(14.0))
                    .gap(px(6.0))
                    .child(Self::field_label("名称", &skin))
                    .child(Input::new(&editor.name).disabled(builtin).w_full())
                    .child(div().h(px(6.0)))
                    .child(Self::field_label("命令正文", &skin))
                    .child(
                        Input::new(&editor.body)
                            .disabled(builtin)
                            .h(px(116.0))
                            .w_full()
                            .font_family("Consolas"),
                    )
                    .child(div().h(px(6.0)))
                    .child(Self::field_label("备注", &skin))
                    .child(Input::new(&editor.notes).h(px(84.0)).w_full())
                    .child(div().h(px(6.0)))
                    .child(Self::field_label("标签（逗号分隔，回车保存）", &skin))
                    .child(Input::new(&editor.tags).w_full())
                    .child(div().h(px(6.0)))
                    .child(Self::field_label("所属分组（可多选；全不选即未分组）", &skin))
                    .child(groups),
            )
            .child(
                Self::action_bar(&skin)
                    .child(div().flex_1())
                    .child(
                        Button::new("cancel-editor")
                            .label("取消")
                            .ghost()
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.back(window, cx))),
                    )
                    .child(
                        Button::new("save-command")
                            .label("保存")
                            .primary()
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| this.save_editor(cx))),
                    ),
            )
            .into_any_element()
    }

    // ----------------------------------------------------------------- detail

    pub(super) fn render_detail(
        &self,
        command: Command,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skin = *style::palette(cx);
        let copy = command.clone();
        let edit = command.clone();
        let duplicate = command.clone();
        let group_names = command
            .groups
            .iter()
            .map(|id| {
                if id == BUILTIN_GROUP {
                    "内置命令".into()
                } else {
                    self.library
                        .groups
                        .iter()
                        .find(|g| &g.id == id)
                        .map(|g| g.name.clone())
                        .unwrap_or_default()
                }
            })
            .collect::<Vec<String>>()
            .join("、");
        v_flex()
            .size_full()
            .child(self.header("命令详情", &skin, cx))
            .child(
                v_flex()
                    .id("details-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(16.0))
                    .py(px(14.0))
                    .gap(px(14.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(tint(skin.fg))
                            .child(command.name.clone()),
                    )
                    .child(
                        div()
                            .w_full()
                            .p(px(12.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(tint(skin.border))
                            .bg(tint(skin.surface))
                            .font_family("Consolas")
                            .text_size(px(13.0))
                            .text_color(tint(skin.fg))
                            .child(command.command.clone()),
                    )
                    .child(
                        v_flex()
                            .gap(px(6.0))
                            .child(Self::field_label("备注", &skin))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(if command.notes.is_empty() {
                                        tint(skin.fg_subtle)
                                    } else {
                                        tint(skin.fg_muted)
                                    })
                                    .child(if command.notes.is_empty() {
                                        "暂无备注".to_string()
                                    } else {
                                        command.notes.clone()
                                    }),
                            ),
                    )
                    .when(!command.tags.is_empty(), |page| {
                        page.child(
                            v_flex()
                                .gap(px(6.0))
                                .child(Self::field_label("标签", &skin))
                                .child(Self::tag_chips(&command.tags, 12, &skin)),
                        )
                    })
                    .child(
                        v_flex()
                            .gap(px(6.0))
                            .child(Self::field_label("分组", &skin))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(tint(skin.fg_muted))
                                    .child(if group_names.is_empty() {
                                        "未分组".to_string()
                                    } else {
                                        group_names
                                    }),
                            ),
                    ),
            )
            .child(
                Self::action_bar(&skin)
                    .child(div().flex_1())
                    .when(command.builtin(), |bar| {
                        bar.child(
                            Button::new("duplicate-builtin")
                                .label("另存副本")
                                .ghost()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.edit(duplicate.clone(), true, window, cx)
                                })),
                        )
                    })
                    .child(
                        Button::new("detail-edit")
                            .label("编辑")
                            .ghost()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.edit(edit.clone(), false, window, cx)
                            })),
                    )
                    .child(
                        Button::new("detail-copy")
                            .label("复制命令")
                            .primary()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.copy(vec![copy.id.clone()], window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    // ----------------------------------------------------------------- manage

    fn manage_row(
        &self,
        skin: &Palette,
        label: String,
        rename_id: String,
        rename_value: String,
        rename_tag: bool,
        delete_id: String,
        delete_tag: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .w_full()
            .h(px(38.0))
            .px(px(8.0))
            .gap(px(4.0))
            .rounded(px(7.0))
            .hover(move |s| s.bg(tint(skin.hover)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.0))
                    .text_color(tint(skin.fg))
                    .child(label),
            )
            .child(
                Button::new(SharedString::from(format!("rename-{rename_id}")))
                    .label("重命名")
                    .small()
                    .ghost()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.text_page(
                            if rename_tag {
                                TextOp::RenameTag(rename_id.clone())
                            } else {
                                TextOp::RenameGroup(rename_id.clone())
                            },
                            rename_value.clone(),
                            window,
                            cx,
                        )
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("remove-{delete_id}")))
                    .icon(Icon::empty().path("icons/trash-2.svg"))
                    .small()
                    .ghost()
                    .tooltip("删除")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.confirm(
                            if delete_tag {
                                Confirmation::Tag(delete_id.clone())
                            } else {
                                Confirmation::Group(delete_id.clone())
                            },
                            cx,
                        )
                    })),
            )
            .into_any_element()
    }

    fn manage_section(&self, skin: &Palette, title: &str, add: Option<AnyElement>) -> Div {
        let mut head = h_flex()
            .w_full()
            .gap(px(6.0))
            .items_center()
            .child(
                div()
                    .text_size(px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tint(skin.fg_subtle))
                    .child(title.to_owned()),
            )
            .child(div().flex_1());
        if let Some(add) = add {
            head = head.child(add);
        }
        v_flex()
            .w_full()
            .p(px(10.0))
            .gap(px(2.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(tint(skin.border))
            .bg(tint(skin.surface))
            .child(head)
    }

    pub(super) fn render_manage(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skin = *style::palette(cx);
        let mut groups = self.manage_section(
            &skin,
            "软件分组",
            Some(
                Button::new("create-group")
                    .icon(IconName::Plus)
                    .label("新增")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.text_page(TextOp::AddGroup, String::new(), window, cx)
                    }))
                    .into_any_element(),
            ),
        );
        if self.library.groups.is_empty() {
            groups = groups.child(
                div()
                    .px(px(8.0))
                    .py(px(10.0))
                    .text_size(px(12.0))
                    .text_color(tint(skin.fg_subtle))
                    .child("还没有分组"),
            );
        }
        for group in &self.library.groups {
            groups = groups.child(self.manage_row(
                &skin,
                group.name.clone(),
                group.id.clone(),
                group.name.clone(),
                false,
                group.id.clone(),
                false,
                cx,
            ));
        }
        let mut tags = self.manage_section(
            &skin,
            "标签",
            Some(
                Button::new("create-tag")
                    .icon(IconName::Plus)
                    .label("新增")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.text_page(TextOp::AddTag, String::new(), window, cx)
                    }))
                    .into_any_element(),
            ),
        );
        if self.library.tags.is_empty() {
            tags = tags.child(
                div()
                    .px(px(8.0))
                    .py(px(10.0))
                    .text_size(px(12.0))
                    .text_color(tint(skin.fg_subtle))
                    .child("还没有标签"),
            );
        }
        for tag in &self.library.tags {
            tags = tags.child(self.manage_row(
                &skin,
                format!("#{tag}"),
                tag.clone(),
                tag.clone(),
                true,
                tag.clone(),
                true,
                cx,
            ));
        }

        v_flex()
            .size_full()
            .child(self.header("管理", &skin, cx))
            .child(
                v_flex()
                    .id("manage-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(14.0))
                    .py(px(12.0))
                    .gap(px(10.0))
                    .child(
                        h_flex()
                            .gap(px(6.0))
                            .child(
                                Button::new("import")
                                    .icon(IconName::ArrowDown)
                                    .label("导入")
                                    .small()
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| this.import(cx))),
                            )
                            .child(
                                Button::new("export")
                                    .icon(IconName::ArrowUp)
                                    .label("导出")
                                    .small()
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| this.export(cx))),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("settings")
                                    .icon(IconName::Settings)
                                    .label("设置")
                                    .small()
                                    .ghost()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_settings(window, cx)
                                    })),
                            ),
                    )
                    .child(groups)
                    .child(tags)
                    .child(
                        v_flex()
                            .w_full()
                            .p(px(10.0))
                            .gap(px(2.0))
                            .rounded(px(10.0))
                            .border_1()
                            .border_color(tint(skin.border))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .pb(px(4.0))
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(tint(skin.danger))
                                    .child("危险操作"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_size(px(12.5))
                                            .text_color(tint(skin.fg_muted))
                                            .child("恢复被隐藏的内置命令"),
                                    )
                                    .child(
                                        Button::new("restore-builtins")
                                            .label("恢复")
                                            .small()
                                            .ghost()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.queue(
                                                    Box::new(|store| {
                                                        Ok(Update::saved(
                                                            store.mutate(|library| {
                                                                library.deleted_builtins.clear();
                                                                Ok(())
                                                            })?,
                                                            "已恢复内置命令",
                                                        ))
                                                    }),
                                                    cx,
                                                )
                                            })),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_size(px(12.5))
                                            .text_color(tint(skin.fg_muted))
                                            .child("清空内置命令，可从本页恢复"),
                                    )
                                    .child(
                                        Button::new("clear-builtins")
                                            .label("清空")
                                            .small()
                                            .ghost()
                                            .text_color(tint(skin.danger))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.confirm(Confirmation::ClearBuiltins, cx)
                                            })),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_size(px(12.5))
                                            .text_color(tint(skin.fg_muted))
                                            .child("删除全部自定义命令"),
                                    )
                                    .child(
                                        Button::new("clear-all")
                                            .label("删除全部…")
                                            .small()
                                            .danger()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.confirm(Confirmation::ClearAll, cx)
                                            })),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    // ------------------------------------------------------------- text input

    pub(super) fn render_text(
        &self,
        op: TextOp,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skin = *style::palette(cx);
        let title = match &op {
            TextOp::AddGroup => "新建分组",
            TextOp::RenameGroup(_) => "重命名分组",
            TextOp::AddTag => "新建标签",
            TextOp::RenameTag(_) => "重命名标签",
        };
        v_flex()
            .size_full()
            .child(self.header(title, &skin, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .px(px(16.0))
                    .py(px(14.0))
                    .gap(px(8.0))
                    .child(Self::field_label("名称", &skin))
                    .child(Input::new(&self.aux).w_full()),
            )
            .child(
                Self::action_bar(&skin)
                    .child(div().flex_1())
                    .child(
                        Button::new("cancel-text")
                            .label("取消")
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| this.back(window, cx))),
                    )
                    .child(
                        Button::new("save-category")
                            .label("保存")
                            .primary()
                            .disabled(self.busy)
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.save_text(op.clone(), cx)),
                            ),
                    ),
            )
            .into_any_element()
    }

    // --------------------------------------------------------------- settings

    pub(super) fn render_settings(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skin = *style::palette(cx);
        let mut appearance = h_flex().gap(px(4.0));
        for choice in [ThemeChoice::Dark, ThemeChoice::Light] {
            appearance = appearance.child(
                Button::new(SharedString::from(format!(
                    "theme-{}",
                    choice.label()
                )))
                .label(choice.label())
                .small()
                .ghost()
                .selected(self.theme == choice)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.theme = choice;
                    style::apply(choice, cx);
                    cx.notify();
                })),
            );
        }
        v_flex()
            .size_full()
            .child(self.header("设置", &skin, cx))
            .child(
                v_flex()
                    .id("settings-fields")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(16.0))
                    .py(px(14.0))
                    .gap(px(6.0))
                    .child(Self::field_label("外观", &skin))
                    .child(appearance)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(tint(skin.fg_subtle))
                            .child("切换立即预览，保存后下次启动继续使用"),
                    )
                    .child(div().h(px(10.0)))
                    .child(Self::field_label("全局快捷键", &skin))
                    .child(Input::new(&self.hotkey).w_full())
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(tint(skin.fg_subtle))
                            .child("例如 Ctrl+Alt+X。冲突时保留原快捷键。"),
                    )
                    .child(div().h(px(10.0)))
                    .child(
                        Checkbox::new("autostart")
                            .label("登录 Windows 后启动并驻留托盘")
                            .checked(self.autostart)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.autostart = *checked;
                                cx.notify();
                            })),
                    )
                    .child(div().h(px(10.0)))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(tint(skin.fg_subtle))
                            .child(format!(
                                "版本 {} · 命令数据保存在程序旁 data 文件夹。",
                                env!("CARGO_PKG_VERSION")
                            )),
                    ),
            )
            .child(
                Self::action_bar(&skin)
                    .child(div().flex_1())
                    .child(
                        Button::new("save-settings")
                            .label("保存设置")
                            .primary()
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| this.save_settings(cx))),
                    ),
            )
            .into_any_element()
    }

    // ----------------------------------------------------------- confirmation

    pub(super) fn render_confirmation(
        &self,
        action: Confirmation,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skin = *style::palette(cx);
        let destructive = !matches!(action, Confirmation::Import(_));
        let text = match &action {
            Confirmation::Commands(ids) => {
                format!("删除 {} 条命令？自定义命令删除后无法在此恢复。", ids.len())
            }
            Confirmation::Group(_) => "删除此分组？命令、备注和标签保留，仅解除该分组关联。".into(),
            Confirmation::Tag(tag) => format!("删除标签 #{tag}？命令和备注保留。"),
            Confirmation::ClearAll => {
                "删除全部自定义命令并清空内置命令？分组保留。建议先导出。".into()
            }
            Confirmation::ClearBuiltins => "清空全部平台的内置命令？可从管理菜单恢复。".into(),
            Confirmation::Import(library) => format!(
                "导入将替换当前命令库：{} 条自定义命令、{} 个分组。请先导出当前数据备份。",
                library.commands.len(),
                library.groups.len()
            ),
        };
        v_flex()
            .size_full()
            .child(self.header("确认操作", &skin, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .px(px(16.0))
                    .py(px(18.0))
                    .gap(px(10.0))
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .items_center()
                            .child(
                                Icon::new(if destructive {
                                    IconName::TriangleAlert
                                } else {
                                    IconName::Info
                                })
                                .size(px(18.0))
                                .text_color(tint(if destructive {
                                    skin.danger
                                } else {
                                    skin.accent
                                })),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(tint(skin.fg))
                                    .child(text),
                            ),
                    ),
            )
            .child(
                Self::action_bar(&skin)
                    .child(div().flex_1())
                    .child(
                        Button::new("cancel-confirm")
                            .label("取消")
                            .ghost()
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.back(window, cx))),
                    )
                    .child(
                        Button::new("confirm-operation")
                            .label(if destructive { "确认删除" } else { "确认导入" })
                            .when(destructive, |button| button.danger())
                            .when(!destructive, |button| button.primary())
                            .disabled(self.busy)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.apply_confirmation(action.clone(), cx)
                            })),
                    ),
            )
            .into_any_element()
    }
}
