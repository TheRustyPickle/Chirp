use rand::Rng;

pub fn get_avatar() -> Vec<u8> {
    let random_num = generate_random_number(5);

    let avatar_url = format!("https://robohash.org/{random_num}.png?size=48x48");

    reqwest::blocking::get(&avatar_url)
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
