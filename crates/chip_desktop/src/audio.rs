use tinyaudio::prelude::*;

pub struct Device {
    inner: Option<Box<dyn BaseAudioOutputDevice>>,
    params: OutputDeviceParameters
}
impl Device {
    pub fn new(params: OutputDeviceParameters) -> Self {
        Self {
            inner: None,
            params
        }
    }
    pub fn beep(&mut self) {
        if self.inner.is_some() { return }
        let params = self.params.clone();
        let device = run_output_device(
            params,
            {
                let mut clock = 0f32;
                move |data| {
                    for samples in data.chunks_mut(params.channels_count) {
                        clock = (clock + 1.0) % params.sample_rate as f32;
                        let val = (clock * 220.0 * 2.0 * std::f32::consts::PI 
                            / params.sample_rate as f32) % (2.0 * std::f32::consts::PI);
                        for sample in samples {
                            *sample = val;
                        }
                    }
                }
            }
        );
        if let Ok(device) = device {
            self.inner = Some(device);
        }
    }
    pub fn stop(&mut self) {
        self.inner.take();
    }
}

pub fn get_device() -> Option<Device> {
    let params = OutputDeviceParameters {
        channels_count: 2,
        sample_rate: 44100,
        channel_sample_count: 4410
    };
    Some(Device::new(params))
}