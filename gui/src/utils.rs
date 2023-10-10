use gio::Cancellable;
use gtk::glib::Bytes;
use rand::Rng;
use soup::{prelude::*, Message, Session};

const COLORS: [&str; 10] = [
    "blue-1", "blue-2", "green-1", "green-2", "yellow-1", "orange-1", "red-1", "purple-1",
    "purple-2", "brown-1",
];
const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

pub fn get_avatar(link: String) -> Result<(String, Bytes), String> {
    let session = Session::new();
    let cancel = Cancellable::new();

    let message = Message::new("GET", &link).map_err(|_| format!("Invalid link"))?;
    let image_data = session
        .send_and_read(&message, Some(&cancel))
        .map_err(|_| format!("Failed to get image data"))?;

    Ok((link, image_data))
}

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

pub fn generate_robohash_link() -> String {
    let random_num = generate_random_string(10);
    let set_num = rand::thread_rng().gen_range(1..5);
    format!("https://robohash.org/{random_num}.svg?set=set{set_num}")
}

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

pub fn generate_multiavatar_link() -> String {
    let random_num = generate_random_string(10);
    format!("https://api.multiavatar.com/{random_num}.svg")
}

// TODO: Perhaps we can add other types of image here
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

pub fn get_random_color(to_ignore: Option<&str>) -> &str {
    let mut colors_vector: Vec<&str> = COLORS.to_vec();

    if let Some(ignore_color) = to_ignore {
        colors_vector.retain(|&color| color != ignore_color);
    }

    let selected_index = rand::thread_rng().gen_range(0..colors_vector.len());
    colors_vector[selected_index]
}
