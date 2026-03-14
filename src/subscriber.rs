// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::log_record::LogRecord;
use crate::metrics::Metrics;
use crate::object_pool::{LOG_RECORD_POOL, STRING_POOL};
use crate::sink::console::ConsoleSink;
use crate::sink::LogSink;
use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

pub struct LoggerSubscriber {
    console_sink: Arc<Mutex<ConsoleSink>>,
    async_sender: Sender<LogRecord>,
    metrics: Arc<Metrics>,
}

impl LoggerSubscriber {
    pub fn new(
        console_sink: Arc<Mutex<ConsoleSink>>,
        async_sender: Sender<LogRecord>,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            console_sink,
            async_sender,
            metrics,
        }
    }
}

impl<S> Layer<S> for LoggerSubscriber
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let record = LogRecord::from_event(event);

        // Fast path: Console
        if let Ok(mut sink) = self.console_sink.lock() {
            if sink.write(&record).is_err() {
                self.metrics.inc_sink_error();
            }
        }

        // Slow path: Async
        match self.async_sender.try_send(record.clone()) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(r)) => {
                self.metrics.inc_channel_blocked();
                if self.async_sender.send(r).is_err() {
                    self.metrics.inc_logs_dropped();
                }
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
        }

        let mut r = record;
        let msg = std::mem::take(&mut r.message);
        STRING_POOL.put(msg);
        LOG_RECORD_POOL.put(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use tracing::subscriber::with_default;
    use tracing_subscriber::prelude::*;

    #[test]
    fn test_on_event_sends_to_channel() {
        let (tx, rx) = bounded(10);
        let console_config = crate::config::ConsoleSinkConfig::default();
        let template = crate::template::LogTemplate::new("{message}");
        let console_sink = Arc::new(Mutex::new(ConsoleSink::new(console_config, template)));
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_sink, tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "hello", user_id = 1u64);
        });

        let received = rx.recv().unwrap();
        assert_eq!(received.level, "INFO");
        assert_eq!(received.target, "test::subscriber");
        assert_eq!(received.message, "hello");
    }
}
