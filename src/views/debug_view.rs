use crate::logger;
use crate::theme;
use crate::vec::Vec2;
use crate::view::View;
use crate::views;
use crate::Printer;

use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq)]
pub enum LogViewFilter {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleFilter {
    All,
    Module,
}

fn record_above_set_filter(
    record_level: log::Level,
    display_level: LogViewFilter,
) -> bool {
    match record_level {
        log::Level::Error => true,
        log::Level::Warn => (display_level != LogViewFilter::Error),
        log::Level::Info => {
            ((display_level == LogViewFilter::Info)
                || (display_level == LogViewFilter::Debug))
        }
        log::Level::Debug => (display_level == LogViewFilter::Debug),
        log::Level::Trace => (display_level == LogViewFilter::Debug),
    }
}


struct DebugLogFilter {}
impl DebugLogFilter {
    fn new(debug_view_id: String) -> views::Panel<views::BoxView<views::ListView>> {
        views::Panel::new(views::BoxView::with_full_width(views::ListView::new().child(
            "Filter Log Levels",
            views::SelectView::new()
                .popup()
                .item("Debug", LogViewFilter::Debug)
                .item("Info", LogViewFilter::Info)
                .item("Warn", LogViewFilter::Warn)
                .item("Error", LogViewFilter::Error)
                .on_submit({
                    move |s, new_filter| {
                        s.call_on_id(&debug_view_id, {
                            move |debug_view: &mut views::DebugView| {
                                debug_view.set_filter(new_filter.clone());
                            }
                        });
                    }
                }),
        )))
    }
}

struct DebugSetLogLevel {}
impl DebugSetLogLevel {
    fn new() -> views::Panel<views::BoxView<views::ListView>> {
        views::Panel::new(views::BoxView::with_full_width(views::ListView::new().child(
            "Set Max Log Level",
            views::SelectView::new()
                .popup()
                .item("Debug", log::LevelFilter::Debug)
                .item("Info", log::LevelFilter::Info)
                .item("Warn", log::LevelFilter::Warn)
                .item("Error", log::LevelFilter::Error)
                .on_submit({
                    move |_s, new_log_level| {
                        log::set_max_level(*new_log_level);
                    }
                }),
        )))
    }
}

struct DebugModFilter {}
impl DebugModFilter {
    fn new(debug_view_id: String) -> views::Panel<views::BoxView<views::ListView>> {
        let mut filter_module_select_view = views::SelectView::new()
                .popup()
                .item("All", ModuleFilter::All)
                .on_submit({
                    move |s, mod_filter| {
                        s.call_on_id(&debug_view_id, {
                            move |debug_view: &mut views::DebugView| {
                                debug_view.set_module(mod_filter.clone());
                            }
                        });
                    }
                });

        let modules = logger::MODULE.lock().unwrap();

        for module in modules.iter() {
            filter_module_select_view.add_item(module.to_string(), ModuleFilter::Module)
        }

        views::Panel::new(views::BoxView::with_full_width(views::ListView::new().child(
            "Filter Log Modules",
            filter_module_select_view
        )))
    }
}

/// View to toggle the logs shown within the debug log console, or update the max log level
pub struct DebugViewFilter {}
impl DebugViewFilter {
    /// Creates a new DebugViewFilter, which filters the logs displayed in the DebugView with the
    /// passed in ID
    pub fn new(debug_view_id: String) -> views::LinearLayout {
        views::LinearLayout::horizontal()
            .child(DebugSetLogLevel::new())
            .child(DebugLogFilter::new(debug_view_id.clone()))
            .child(DebugModFilter::new(debug_view_id.clone()))
    }
}

/// View used for debugging, showing logs.
pub struct DebugView {
    filter: LogViewFilter,
    module: ModuleFilter
    // TODO: wrap log lines if needed, and save the line splits here.
}

impl DebugView {
    /// Creates a new DebugView.
    pub fn new() -> Self {
        DebugView {
            filter: LogViewFilter::Debug,
            module: ModuleFilter::All
        }
    }

    /// Updates the maximum log level of logs displayed within the DebugView
    fn set_filter(&mut self, new_filter: LogViewFilter) {
        self.filter = new_filter;
    }

    /// Updates the maximum log level of logs displayed within the DebugView
    fn set_module(&mut self, new_module: ModuleFilter) {
        self.module = new_module;
    }
}

impl Default for DebugView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for DebugView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let logs = match self.module {
            ModuleFilter::All => logger::LOGS.lock().unwrap(),
            ModuleFilter::Module => logger::MODULE_LOGS.lock().unwrap()
        };

        // Only print the last logs, so skip what doesn't fit
        let skipped = logs.len().saturating_sub(printer.size.y);

        let mut i = 0;

        for record in logs.iter().skip(skipped) {
            if record_above_set_filter(record.level, self.filter.clone()) {
                // TODO: Apply style to message? (Ex: errors in bold?)
                // TODO: customizable time format? (24h/AM-PM)
                printer.print(
                    (0, i),
                    &format!(
                        "{} | [     ] | {} | {}",
                        record.time.with_timezone(&chrono::Local).format("%T%.3f"),
                        record.target,
                        record.message
                    ),
                );
                let color = match record.level {
                    log::Level::Error => theme::BaseColor::Red.dark(),
                    log::Level::Warn => theme::BaseColor::Yellow.dark(),
                    log::Level::Info => theme::BaseColor::Black.light(),
                    log::Level::Debug => theme::BaseColor::Green.dark(),
                    log::Level::Trace => theme::BaseColor::Blue.dark(),
                };
                printer.with_color(color.into(), |printer| {
                    printer.print((16, i), &format!("{:5}", record.level))
                });

                i += 1;
            }
        }
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        // TODO: read the logs, and compute the required size to print it.
        let logs = logger::LOGS.lock().unwrap();

        let level_width = 7; // Width of "[ERROR]"
        let time_width = 12; // Width of "23:59:59.123"
        let separator_width = 3; // Width of " | "

        // The longest line sets the width
        let w = logs
            .iter()
            .map(|record| {
                record.message.width()
                    + record.target.width()
                    + level_width
                    + time_width
                    + separator_width * 3
            })
            .max()
            .unwrap_or(1);
        let h = logs.len();

        Vec2::new(w, h)
    }

    fn layout(&mut self, _size: Vec2) {
        // Uh?
    }
}
