mod message_data;
mod message_row;
mod user_data;
mod user_row;
mod utils;
mod window;

use window::Window;

use adw::Application;
use gdk::Display;
use gio::resources_register_include;
use glib::ExitCode;
use gtk::{gdk, prelude::*, CssProvider};
use gtk::{gio, glib};

const APP_ID: &str = "com.github.therustypickle.chirp";

fn main() -> ExitCode {
    resources_register_include!("chirp.gresource").expect("Could not load gresource");

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_app| load_css());
    app.connect_activate(build_ui);
    app.set_accels_for_action("win.send-message", &["<Primary>Return"]);
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
