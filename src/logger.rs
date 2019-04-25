//! Logging utilities

use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Saves all log records in a global deque.
///
/// Uses a `DebugView` to access it.
struct CursiveLogger;

static LOGGER: CursiveLogger = CursiveLogger;

/// A log record.
pub struct Record {
    /// Log level used for this record
    pub level: log::Level,
    /// Module that logged this message
    pub target: String,
    /// Time this message was logged
    pub time: chrono::DateTime<chrono::Utc>,
    /// Message content
    pub message: String,
}

lazy_static! {
    /// The name of a module, which the user can filter logs for using a `DebugFilter`
    /// Only initiated if the user calls `init_for_module`
    pub static ref MODULE: Mutex<Option<String>> =
        Mutex::new(None);
}

lazy_static! {
    /// Circular buffer for logs relating to a custom module.
    /// The user can filter logs using a `DebugFilter`
    pub static ref MODULE_LOGS: Mutex<VecDeque<Record>> =
        Mutex::new(VecDeque::new());
}

lazy_static! {
    /// Circular buffer for logs. Use it to implement `DebugView`.
    pub static ref LOGS: Mutex<VecDeque<Record>> =
        Mutex::new(VecDeque::new());
}

fn log_record_to(record: &log::Record<'_>, log_buffer: &Mutex<VecDeque<Record>>) {
    let mut logs = log_buffer.lock().unwrap();

    // TODO: customize the format? Use colors? Save more info?
    if logs.len() == logs.capacity() {
        logs.pop_front();
    }

    //  Only display the high-level module
    let record_target = record.target().split("::").collect::<Vec<&str>>()[0];
    logs.push_back(Record {
        level: record.level(),
        target: format!("{}", record_target),
        message: format!("{}", record.args()),
        time: chrono::Utc::now(),
    });
}

impl log::Log for CursiveLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        log_record_to(&record, &LOGS);

        let custom_module = MODULE.lock().unwrap();
        //  If the logger has been configured with the ability to filter logs for a specific
        //  module, and this log is from said module, add it to the module logs circular buffer
        match *custom_module {
            Some(ref module_name) => {
                if record.target().split("::").collect::<Vec<&str>>()[0] == module_name {
                    log_record_to(&record, &MODULE_LOGS)
                }
            }
            None => return
        }
    }

    fn flush(&self) {}
}

/// Initialize the Cursive logger.
///
/// Make sure this is the only logger your are using.
///
/// Use a [`DebugView`](crate::views::DebugView) to see the logs, or use
/// [`Cursive::toggle_debug_console()`](crate::Cursive::toggle_debug_console()).
pub fn init() {
    // TODO: Configure the deque size?
    LOGS.lock().unwrap().reserve(1_000);

    // This will panic if `set_logger` was already called.
    log::set_logger(&LOGGER).unwrap();

    // TODO: read the level from env variable? From argument?
    log::set_max_level(log::LevelFilter::Trace);
}

/// Initialise the Cursive logger, adding the ability to filter debug logs by module
pub fn init_for_module(module: String) {
    let mut custom_module = MODULE.lock().unwrap();
    *custom_module = Some(module);

    // TODO: Configure the deque size?
    MODULE_LOGS.lock().unwrap().reserve(1_000);

    init();
}
