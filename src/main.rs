#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod platform;
mod resources;
mod style;
mod ui;
mod worker;

use gpui::{AppContext as _, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_component::Root;

fn main() {
    if let Err(error) = run() {
        rfd::MessageDialog::new()
            .set_title("命令管理器启动失败")
            .set_description(format!("{error:#}"))
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}

fn run() -> anyhow::Result<()> {
    let Some(singleton) = platform::Singleton::acquire()? else {
        return Ok(());
    };
    let mut args = std::env::args().skip(1);
    let mut directory = std::env::current_exe()?.parent().unwrap().join("data");
    let mut background = false;
    let mut stay_open = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--background" => background = true,
            "--stay-open" => stay_open = true,
            "--data-dir" => {
                directory = std::path::PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--data-dir 缺少路径"))?,
                )
            }
            _ => {}
        }
    }
    std::fs::create_dir_all(&directory)?;
    gpui_platform::application()
        .with_assets(resources::Assets)
        .run(move |cx| {
            let desktop = match platform::Desktop::new(&directory) {
                Ok(desktop) => desktop,
                Err(error) => {
                    rfd::MessageDialog::new()
                        .set_title("托盘初始化失败")
                        .set_description(error.to_string())
                        .show();
                    cx.quit();
                    return;
                }
            };
            style::init(cx, desktop.settings.theme);
            let bounds = Bounds::centered(
                None,
                size(px(style::WINDOW_WIDTH), px(280.0)),
                cx,
            );
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(400.0), px(260.0))),
                titlebar: None,
                show: !background,
                focus: !background,
                // A resizable frame adds an invisible border and a drop shadow on
                // Windows; the panel sizes itself instead.
                is_resizable: false,
                app_id: Some("standalone-command-manager".into()),
                ..Default::default()
            };
            let result = cx.open_window(options, move |window, cx| {
                window.set_window_title("命令管理器");
                platform::set_window_icon(window);
                platform::make_frameless(window);
                let view = cx.new(|cx| {
                    ui::Manager::new(directory, desktop, singleton, stay_open, window, cx)
                });
                cx.new(|cx| Root::new(view, window, cx))
            });
            if let Err(error) = result {
                rfd::MessageDialog::new()
                    .set_title("窗口创建失败")
                    .set_description(error.to_string())
                    .show();
                cx.quit();
            }
            if background {
                // 静默启动时窗口从未显示过，也就没走过 hide()；
                // 这里直接释放一次工作集，托盘常驻保持个位数内存。
                platform::release_working_set();
            } else {
                cx.activate(true);
            }
        });
    Ok(())
}
