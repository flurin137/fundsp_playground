//! Make real-time changes to a network while it is playing.
#![allow(clippy::precedence)]

use assert_no_alloc::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use fundsp::hacker::*;

#[cfg(debug_assertions)] // required when disable_release is set (default)
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

struct Note {
    note: BaseNote,
    octave: i32,
}

impl Note {
    fn base(note: BaseNote) -> Self {
        Self { note, octave: 0 }
    }

    fn new(note: BaseNote, octave: i32) -> Self {
        Self { note: note, octave }
    }
}

struct Accord {
    notes: Vec<Note>,
}

enum BaseNote {
    C,
    Cis,
    D,
    Dis,
    E,
    F,
    Fis,
    G,
    Gis,
    A,
    Ais,
    H,
}

fn main() {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()).unwrap(),
        _ => panic!("Unsupported format"),
    }
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<f64>,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let bpm = 160;

    let mut net = Net64::new(0, 2);

    let id_noise = net.chain(Box::new(zero()));
    net.chain(Box::new(pan(0.0)));

    net.set_sample_rate(sample_rate);

    let mut backend = net.backend();

    let mut next_value = move || assert_no_alloc(|| backend.get_stereo());

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    use BaseNote::*;

    let a = vec![
        (Note { note: C, octave: 0 }, 1),
        (Note { note: D, octave: 0 }, 1),
        (Note { note: E, octave: 0 }, 1),
        (Note { note: F, octave: 0 }, 1),
        (Note { note: G, octave: 0 }, 2),
        (Note { note: G, octave: 0 }, 2),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: G, octave: 0 }, 2),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: A, octave: 0 }, 1),
        (Note { note: G, octave: 0 }, 2),
        (Note { note: F, octave: 0 }, 1),
        (Note { note: F, octave: 0 }, 1),
        (Note { note: F, octave: 0 }, 1),
        (Note { note: F, octave: 0 }, 1),
        (Note { note: E, octave: 0 }, 2),
        (Note { note: E, octave: 0 }, 2),
        (Note { note: D, octave: 0 }, 1),
        (Note { note: D, octave: 0 }, 1),
        (Note { note: D, octave: 0 }, 1),
        (Note { note: D, octave: 0 }, 1),
        (Note { note: C, octave: 0 }, 3),
    ];

    let asdf = vec![Accord {
        notes: vec![Note::new(C, 0), Note::new(E, 0), Note::new(C, 1)],
    }];

    for accord in asdf {
        let length = 4 * 60000 / bpm;

        let frequencies = accord.notes.iter().map(get_note_frequency);

        for frequency in frequencies {
            let c = zero() >> pluck(frequency, 0.5, 0.9);

            net.replace(id_noise, Box::new(c));

            net.commit();
        }

        std::thread::sleep(std::time::Duration::from_millis(length));
    }
    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
where
    T: SizedSample + FromSample<f64>,
{
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left = T::from_sample(sample.0);
        let right: T = T::from_sample(sample.1);

        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = left;
            } else {
                *sample = right;
            }
        }
    }
}

fn get_note_frequency(note: &Note) -> f64 {
    let note_number = match note.note {
        BaseNote::C => 0,
        BaseNote::Cis => 1,
        BaseNote::D => 2,
        BaseNote::Dis => 3,
        BaseNote::E => 4,
        BaseNote::F => 5,
        BaseNote::Fis => 6,
        BaseNote::G => 7,
        BaseNote::Gis => 8,
        BaseNote::A => 9,
        BaseNote::Ais => 10,
        BaseNote::H => 11,
    } as f64;

    let note = note_number + 12.0 * note.octave as f64;

    440.0 * 2.0.pow((note - 9.0) / 12.0) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn test_get_note_frequency_c() {
        assert_approx_eq!(
            get_note_frequency(&Note {
                note: BaseNote::C,
                octave: 0
            }),
            261.626,
            0.01
        );
    }

    #[test]
    fn test_get_note_frequency_a() {
        assert_eq!(
            get_note_frequency(&Note {
                note: BaseNote::A,
                octave: 0
            }),
            440.0
        );
    }
}
