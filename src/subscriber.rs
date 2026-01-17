use crate::log_record::LogRecord;
use crate::metrics::Metrics;
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
        use crate::pool::{LOG_RECORD_POOL, STRING_POOL};
        let record = LogRecord::from_event(event);

        // Fast path: Console
        if let Ok(mut sink) = self.console_sink.lock() {
            if sink.write(&record).is_err() {
                self.metrics.inc_sink_error();
            }
        }

        // Slow path: Async
        // We try send first to avoid blocking if possible, but for "zero loss" we might block
        // PRD says "Bounded Channel + Backpressure Block"
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

        // Return resources to pools
        let mut r = record;
        let msg = std::mem::take(&mut r.message);
        STRING_POOL.put(msg);
        LOG_RECORD_POOL.put(r);
    }
}
