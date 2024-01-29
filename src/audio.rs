pub const SAMPLE_RATE: u32 = 24000;

pub struct Audio;

impl ludus::AudioDevice for Audio {
    fn push_sample(&mut self, _sample: f32) {}
}