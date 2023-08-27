use rand::Rng;
use tracing::info;

const COLORS: [&str; 10] = [
    "blue-1", "blue-2", "green-1", "green-2", "yellow-1", "orange-1", "red-1", "purple-1",
    "purple-2", "brown-1",
];

pub fn get_avatar(link: String) -> Vec<u8> {
    info!("Starting fetching avatar...");
    reqwest::blocking::get(&link)
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec()
}

fn generate_random_number(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(10u32.pow(length as u32 - 1)..10u32.pow(length as u32));
    info!("Random generated number: {}", num);
    num.to_string()
}

pub fn generate_robohash_link() -> String {
    let random_num = generate_random_number(5);
    format!("https://robohash.org/{random_num}.png")
}

pub fn generate_dicebear_link() -> String {
    let random_num = generate_random_number(5);
    format!("https://api.dicebear.com/6.x/micah/svg?seed={random_num}")
}

pub fn get_random_color(to_ignore: Option<&str>) -> &str {
    let mut colors_vector: Vec<&str> = COLORS.to_vec();

    if to_ignore.is_some() {
        colors_vector.retain(|&color| color != to_ignore.unwrap());
    }

    let selected_index = rand::thread_rng().gen_range(0..colors_vector.len());
    info!("Color chosen: {}", colors_vector[selected_index]);
    colors_vector[selected_index]
}
