use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample, SampleFormat, Stream
};

pub struct Device {
    inner: cpal::Device,
    config: cpal::StreamConfig,
    stream: Option<Stream>
}
impl Device {
    pub fn new(device: cpal::Device, config: cpal::SupportedStreamConfig) -> Self {
        Self {
            inner: device,
            config: config.into(),
            stream: None
        }
    }
    pub fn beep(&mut self) {
        if self.stream.is_some() { return }
        let sample_rate = self.config.sample_rate.0 as f32;
        let channels = self.config.channels as usize;

        let mut clock = 0f32;
        let mut next_value = move || {
            clock = (clock + 1.0) % sample_rate;
            // (clock * 220.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
            (clock * 220.0 * 2.0 * std::f32::consts::PI / sample_rate) % (2.0 * std::f32::consts::PI)
        };
    
        let stream = self.inner.build_output_stream(
            &self.config,
            move |data: &mut [f32], _| write(data, channels, &mut next_value),
            |_| {},
            None
        ).unwrap();
        self.stream = Some(stream);
        self.stream.as_mut().unwrap().play().unwrap();
    }
    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
    }
}

pub fn get_device() -> Option<Device> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    Some(Device::new(device, config))
}

fn write(
    output: &mut [f32],
    channels: usize,
    next_sample: &mut dyn FnMut() -> f32
) {
    for frame in output.chunks_mut(channels) {
        let val = f32::from_sample(next_sample());
        for sample in frame.iter_mut() {
            *sample = val;
        }
    }
}
