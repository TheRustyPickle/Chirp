mod message;
mod user;
mod utils;
mod window;
mod ws;

use adw::Application;
use gdk::Display;
use gio::resources_register_include;
use glib::ExitCode;
use gtk::{gdk, gio, glib, prelude::*, CssProvider};
use tracing::info;
use window::Window;

const APP_ID: &str = "com.github.therustypickle.chirp";

fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    resources_register_include!("chirp.gresource").expect("Could not load gresource");

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_app| load_css());
    app.connect_activate(build_ui);
    app.set_accels_for_action("win.send-message", &["<Primary>Return"]);
    info!("Starting the app");
    app.run()
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    window.present();
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_resource("/com/github/therustypickle/chirp/style.css");

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
