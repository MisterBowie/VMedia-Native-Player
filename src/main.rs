mod app;
mod core;
mod infra;
mod player;
mod ui;

fn main() -> gtk::glib::ExitCode {
    // SAFETY: GTK recommends forcing LC_NUMERIC to "C" before initialization to avoid locale-dependent parsing issues.
    unsafe {
        libc::setenv(c"LC_NUMERIC".as_ptr(), c"C".as_ptr(), 1);
        libc::setlocale(libc::LC_NUMERIC, c"C".as_ptr());
    }

    infra::logging::init();
    app::run()
}
