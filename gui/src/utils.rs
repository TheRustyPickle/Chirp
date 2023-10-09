use gio::Cancellable;
use gtk::glib::Bytes;
use rand::Rng;
use soup::{prelude::*, Message, Session};
use tracing::info;

const COLORS: [&str; 10] = [
    "blue-1", "blue-2", "green-1", "green-2", "yellow-1", "orange-1", "red-1", "purple-1",
    "purple-2", "brown-1",
];

pub fn get_avatar(link: String) -> Result<(String, Bytes), String> {
    info!("Starting fetching avatar...");
    let session = Session::new();
    let cancel = Cancellable::new();

    let message = Message::new("GET", &link).map_err(|_| format!("Invalid link"))?;
    let image_data = session
        .send_and_read(&message, Some(&cancel))
        .map_err(|_| format!("Failed to get image data"))?;

    Ok((link, image_data))
}

fn generate_random_number(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(10u32.pow(length as u32 - 1)..10u32.pow(length as u32));
    info!("Random generated number: {}", num);
    num.to_string()
}

pub fn generate_robohash_link() -> String {
    let random_num = generate_random_number(5);
    let set_num = rand::thread_rng().gen_range(1..5);
    format!("https://robohash.org/{random_num}.svg?set=set{set_num}")
}

pub fn generate_dicebear_link() -> String {
    let choices = [
        "micah",
        "bottts",
        "lorelei",
        "adventurer",
        "open-peeps",
        "bottts-neutral",
        "notionists",
        "rings",
        "shapes",
    ];

    let random_index = rand::thread_rng().gen_range(0..choices.len());
    let selected_choice = choices[random_index];

    let random_num = generate_random_number(5);
    format!("https://api.dicebear.com/7.x/{selected_choice}/svg?seed={random_num}")
}

pub fn generate_multiavatar_link() -> String {
    let random_num = generate_random_number(5);
    format!("https://api.multiavatar.com/{random_num}.svg")
}

// TODO: Perhaps we can add other types of image here
// NOTE Identicon
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
    info!("Color chosen: {}", colors_vector[selected_index]);
    colors_vector[selected_index]
}
