use chrono::{Local, NaiveDateTime};
use gio::Cancellable;
use gtk::glib::Bytes;
use rand::Rng;
use soup::prelude::*;
use soup::{Message, Session};

const COLORS: [&str; 10] = [
    "blue-1", "blue-2", "green-1", "green-2", "yellow-1", "orange-1", "red-1", "purple-1",
    "purple-2", "brown-1",
];
const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Try to fetch image bytes from a given URL
pub fn get_avatar(link: String) -> Result<(String, Bytes), String> {
    let session = Session::new();
    let cancel = Cancellable::new();

    let message = Message::new("GET", &link).map_err(|_| format!("Invalid link"))?;
    let image_data = session
        .send_and_read(&message, Some(&cancel))
        .map_err(|_| format!("Failed to get image data"))?;

    Ok((link, image_data))
}

/// Generates a random string
fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let result: String = (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    result
}

/// Generates a robohash api link with a random string
pub fn generate_robohash_link() -> String {
    let random_num = generate_random_string(10);
    let set_num = rand::thread_rng().gen_range(1..5);
    format!("https://robohash.org/{random_num}.svg?set=set{set_num}")
}

/// Generates a dicebear api link with a random string
pub fn generate_dicebear_link() -> String {
    let choices = [
        "avataaars",
        "big-smile",
        "micah",
        "bottts",
        "lorelei",
        "adventurer",
        "open-peeps",
        "bottts-neutral",
        "notionists",
        "rings",
        "shapes",
        "thumbs",
    ];

    let random_index = rand::thread_rng().gen_range(0..choices.len());
    let selected_choice = choices[random_index];

    let random_num = generate_random_string(10);
    format!("https://api.dicebear.com/7.x/{selected_choice}/svg?seed={random_num}")
}

/// Generates a multiavatar api link with a random string
pub fn generate_multiavatar_link() -> String {
    let random_num = generate_random_string(10);
    format!("https://api.multiavatar.com/{random_num}.svg")
}

// TODO: Perhaps we can add other types of image here
/// Generates a random supported image api link
pub fn generate_random_avatar_link() -> String {
    let choices = ["dicebear", "robohash", "multiavatar"];

    let random_index = rand::thread_rng().gen_range(0..choices.len());
    let selected_choice = choices[random_index];

    match selected_choice {
        "dicebear" => generate_dicebear_link(),
        "robohash" => generate_robohash_link(),
        "multiavatar" => generate_multiavatar_link(),
        _ => unreachable!(),
    }
}

/// Selects a random css color class name. Used for created owner UserObject
pub fn get_random_color(to_ignore: Option<&str>) -> &str {
    let mut colors_vector: Vec<&str> = COLORS.to_vec();

    if let Some(ignore_color) = to_ignore {
        colors_vector.retain(|&color| color != ignore_color);
    }

    let selected_index = rand::thread_rng().gen_range(0..colors_vector.len());
    colors_vector[selected_index]
}

/// Compare the current date with the given date to determine how the time should be shown in the UI
// target_date is always in Local time
pub fn get_created_at_timing(target_date: &NaiveDateTime) -> String {
    let now = Local::now().naive_local().date();

    let naive_date = target_date.date();

    let naive_time = target_date.time().format("%I:%M %p").to_string();

    if now == naive_date {
        format!("Today at {}", naive_time)
    } else if now == naive_date.pred_opt().unwrap() {
        format!("Yesterday at {}", naive_time)
    } else {
        format!("{} {}", naive_date, naive_time)
    }
}
