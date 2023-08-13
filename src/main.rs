mod message_data;
mod message_row;
mod window;

use window::Window;

use adw::Application;
use gio::resources_register_include;
use glib::ExitCode;
use gtk::{gdk, prelude::*};
use gtk::{gio, glib};

const APP_ID: &str = "com.github.therustypickle.chirp";

fn main() -> ExitCode {
    resources_register_include!("chirp.gresource").expect("Could not load gresource");

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    window.present();
}
