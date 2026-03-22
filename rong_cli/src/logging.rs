use tracing::{Event, Level, Subscriber, field::Field, field::Visit};
use tracing_subscriber::{
    EnvFilter, Registry,
    filter::filter_fn,
    layer::{Context, Layer, SubscriberExt},
    util::SubscriberInitExt,
};

#[derive(Default)]
struct ConsoleMessageVisitor {
    message: Option<String>,
    namespace: Option<String>,
    scope: Option<String>,
}

impl Visit for ConsoleMessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = Some(value.to_string()),
            "namespace" => self.namespace = Some(value.to_string()),
            "scope" => self.scope = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(format!("{value:?}"));
        }
    }
}

struct JsConsoleLayer;

impl<S> Layer<S> for JsConsoleLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != "rong.js.console" {
            return;
        }

        let mut visitor = ConsoleMessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message.unwrap_or_default();
        let mut prefix = String::new();
        if let Some(namespace) = visitor.namespace {
            prefix.push('[');
            prefix.push_str(&namespace);
            prefix.push(']');
        }
        if let Some(scope) = visitor.scope {
            prefix.push('[');
            prefix.push_str(&scope);
            prefix.push(']');
        }
        let rendered = if prefix.is_empty() {
            message
        } else {
            format!("{prefix} {message}")
        };

        match *event.metadata().level() {
            Level::ERROR => eprintln!("ERROR: {}", rendered),
            Level::WARN => eprintln!("WARN: {}", rendered),
            Level::DEBUG => println!("DEBUG: {}", rendered),
            _ => println!("{}", rendered),
        }
    }
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_env("RONG_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("warn,rong.js.console=info,rong=warn"));

    Registry::default()
        .with(filter)
        .with(JsConsoleLayer.with_filter(filter_fn(|meta| meta.target() == "rong.js.console")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(filter_fn(|meta| meta.target() != "rong.js.console")),
        )
        .init();
}
