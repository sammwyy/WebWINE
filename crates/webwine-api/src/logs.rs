use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub pid: Option<u32>,
}

pub struct LogBuffer {
    events: VecDeque<LogEvent>,
    max_events: usize,
}

impl LogBuffer {
    pub fn new(max_events: usize) -> Self {
        LogBuffer {
            events: VecDeque::new(),
            max_events,
        }
    }

    pub fn push(&mut self, event: LogEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn log(&mut self, level: LogLevel, target: &str, message: &str, pid: Option<u32>) {
        self.push(LogEvent {
            level,
            target: target.to_string(),
            message: message.to_string(),
            pid,
        });
    }

    pub fn drain(&mut self) -> Vec<LogEvent> {
        self.events.drain(..).collect()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(1024)
    }
}
