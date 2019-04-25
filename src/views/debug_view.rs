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

/// View used to toggle the log-levels shown within the debug log console
pub struct DebugFilter {}

impl DebugFilter {
    /// Creates a new DebugFilter, acting on the DebugView with the passed in view ID
    pub fn new(debug_view_id: String) -> views::LinearLayout {
        let filter_view = views::Panel::new(views::BoxView::with_full_width(views::ListView::new().child(
            "Filter Logs",
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
        )));

        let set_loglevel_view = views::Panel::new(views::BoxView::with_full_width(views::ListView::new().child(
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
        )));

        views::LinearLayout::horizontal()
            .child(set_loglevel_view)
            .child(filter_view)
    }
}

/// View used for debugging, showing logs.
pub struct DebugView {
    filter: LogViewFilter,
    // TODO: wrap log lines if needed, and save the line splits here.
}

impl DebugView {
    /// Creates a new DebugView.
    pub fn new() -> Self {
        DebugView {
            filter: LogViewFilter::Debug,
        }
    }

    /// Updates the maximum log level of logs displayed within the DebugView
    fn set_filter(&mut self, new_filter: LogViewFilter) {
        self.filter = new_filter;
    }
}

impl Default for DebugView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for DebugView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let logs = logger::LOGS.lock().unwrap();
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
                        record
                            .time
                            .with_timezone(&chrono::Local)
                            .format("%T%.3f"),
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
