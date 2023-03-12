use std::collections::HashMap;

pub struct Sound {
    sounds: HashMap<String, macroquad::audio::Sound>,
}

impl Sound {
    pub async fn new() -> Self {
        let mut sounds = HashMap::new();
        sounds.insert("thud".to_owned(), macroquad::audio::load_sound("assets/thud.wav").await.unwrap());
        sounds.insert("thud2".to_owned(), macroquad::audio::load_sound("assets/thud2.wav").await.unwrap());
        sounds.insert("thud3".to_owned(), macroquad::audio::load_sound("assets/thud3.wav").await.unwrap());
        sounds.insert("take".to_owned(), macroquad::audio::load_sound("assets/take.wav").await.unwrap());
        sounds.insert("stairs".to_owned(), macroquad::audio::load_sound("assets/stairs.wav").await.unwrap());
        Sound {
            sounds,
        }
    }

    pub fn play(&self, s: &str) {
        macroquad::audio::play_sound_once(*self.sounds.get(s).unwrap());
    }
}