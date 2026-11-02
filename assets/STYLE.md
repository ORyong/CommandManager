# 样式来源与独立实现

`src/style.rs` 是应用唯一的配色与尺寸来源；界面布局位于 `src/ui/views.rs`。这里没有 CSS。
`views.rs` 不允许出现颜色字面量，所有取色经 `style::palette(cx)` 与 `style::tint()`。

## 主题

两套 `Palette`：`DARK` 沿用上游 Nord，`LIGHT` 为独立设计的纸白主题。
切换时 `style::apply` 同时更新 `Palette`（自绘部分）与 `gpui_component::Theme`（按钮、输入框、菜单、滚动条），
最后重建 `theme.tokens`。

| 角色 | 深色 Dark | 浅色 Light |
|---|---|---|
| window / panel | #23272F / #2B313C | #E6E8EC / #FFFFFF |
| surface / raised | #333A47 / #383F4D | #F5F6F8 / #E8EBF0 |
| border / border_strong | #FFFFFF12 / #454E60 | #0000000F / #D6DAE0 |
| fg / muted / subtle | #E7EBF3 / #AEB7C6 / #7C8798 | #1B2027 / #59616F / #8A929F |
| accent / accent_soft | #88C0D0 / #88C0D024 | #2E7A8C / #2E7A8C14 |
| hover / selected / copied | #FFFFFF09 / #88C0D01F / #A3BE8C1F | #00000006 / #2E7A8C12 / #3C7A4B12 |
| tag_bg / tag_fg | #88C0D016 / #9FD2DE | #2E7A8C12 / #26697A |
| success / danger / warning | #A3BE8C / #BF616A / #EBCB8B | #3C7A4B / #B23B45 / #9A6B12 |

浅色主题的强调色比深色深两档，否则白底上对比度不足；两个主题的文字对背景对比度均不低于 4.5:1。

## 尺寸

| 项 | 值 |
|---|---|
| 面板宽 | 520（窗口即面板，无外圈留白） |
| 搜索区 / 工具栏高 | 40 / 40 |
| 分组行 / 命令行高 | 60 / 60 |
| 页脚高 | 46 |
| 圆角 | 面板 0（窗口不透明，圆角会露出底色）、控件 7、胶囊 5 |

行高统一后列表呈网格感；命令正文固定一行，超出部分截断，完整内容在详情页。

## 来源

深色主题沿用 Nord 色系（#2e3440 / #3b4252 / #e5e9f0 / #88c0d0 等），在此基础上压暗面板、
提亮文字并补充三级文字与状态色。浅色主题为本项目自定义。
输入框、按钮、复选框和图标沿用同一固定提交的 gpui-component。

沿用 Geist 字体（已内嵌，OFL 许可同目录），中文通过系统字体回退；命令正文与代码块使用 Consolas。
默认面板宽度 520 logical px，`titlebar=None`，不显示系统顶部栏。
应用图标为独立终端图形，源稿 `assets/app-icon.svg`，构建使用多尺寸 `assets/app.ico`，托盘使用同源 32px RGBA。
