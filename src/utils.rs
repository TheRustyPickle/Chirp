use rand::Rng;

pub fn get_avatar(link: String) -> Vec<u8> {
    reqwest::blocking::get(&link)
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec()
}

fn generate_random_number(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(10u32.pow(length as u32 - 1)..10u32.pow(length as u32));
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
