mod backend;
mod components;
mod model;
mod paths;
mod theme;
mod view;

use backend::Backend;
use model::{State, Wake};
use rune::{App, AppSpec};
use std::sync::{Arc, Mutex, mpsc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rune::orderly_exit();
    rune::set_app_id("com.opul.rune");
    let paths = paths::Paths::discover()?;
    eprintln!(
        "Rune data: {}\nRune downloads: {}\nRune resources: {:?}",
        paths.data.display(),
        paths.downloads.display(),
        paths.resources
    );
    let (events, receiver) = mpsc::channel();
    let wake: Wake = Arc::new(Mutex::new(None));
    let worker_wake = wake.clone();
    let backend = Backend::start(
        paths,
        events,
        Arc::new(move || {
            if let Some(wake) = worker_wake
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                wake();
            }
        }),
    )?;
    let state = State::new(backend.client.clone(), receiver, wake);
    let app = App::new(AppSpec {
        min_size: Some((900.0, 540.0)),
        setup: Some(|s: &mut State, wake| {
            *s.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(wake)
        }),
        poll: Some(|s, _, _| s.poll()),
        ..AppSpec::new(
            "Opul",
            (1180.0, 760.0),
            || unreachable!("state supplied by App::init"),
            view::view,
        )
    })
    .init(move || state)
    .standard_menus()
    .on_quit(|s, _| s.shutdown())
    .on_close(|s, _| s.shutdown());
    let result = app.try_run();
    backend.finish()?;
    result?;
    Ok(())
}

#[cfg(test)]
mod tests;
