use leptos::*;

#[derive(thiserror::Error, Debug)]
#[error("no global tracing subscriber set")]
struct NoTracingSubscriber;

fn configure_tracing() -> anyhow::Result<()> {
	let result = Err(NoTracingSubscriber);

	#[cfg(target_arch = "wasm32")]
	let result = result.or_else(|_| tracing_wasm::try_set_as_global_default());

	let result = result.or_else(|_| {
		let max_level = if cfg!(debug_assertions) {
			tracing::Level::TRACE
		} else {
			tracing::Level::INFO
		};
		tracing::subscriber::set_global_default(
			tracing_subscriber::FmtSubscriber::builder()
				.with_max_level(max_level)
				.finish(),
		)
	});

	Ok(result?)
}

fn configure_logging() -> anyhow::Result<()> {
	configure_tracing()?;

	// Redirect `log` to `tracing`. Another option would be to redirect `tracing` to `log`. Because we enable the "log" feature on the `tracing` crate, that's exactly what will happen if we fail to set `tracing`s global default subscriber above.
	Ok(tracing_log::LogTracer::init()?)
}

fn main() {
	#[cfg(target_arch = "wasm32")]
	console_error_panic_hook::set_once();

	if let Err(error) = configure_logging() {
		// We can technically continue without logging.
		tracing::error!(error = error.to_string());
	}

	mount_to_body(stark::App)
}
