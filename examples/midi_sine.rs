//! Creates a jack midi input and output ports. The application prints
//! out all values sent to it through the input port. It also sends a
//! Note On and Off event, once every cycle, on the output port.
use crossbeam_channel::bounded;
use std::convert::From;
use std::io;
use std::sync::mpsc::sync_channel;

const MAX_MIDI: usize = 3;

//a fixed size container to copy data out of real-time thread
#[derive(Copy, Clone)]
struct MidiCopy {
    len: usize,
    data: [u8; MAX_MIDI],
    time: jack::Frames,
}

impl From<jack::RawMidi<'_>> for MidiCopy {
    fn from(midi: jack::RawMidi<'_>) -> Self {
        let len = std::cmp::min(MAX_MIDI, midi.bytes.len());
        let mut data = [0; MAX_MIDI];
        data[..len].copy_from_slice(&midi.bytes[..len]);
        MidiCopy {
            len,
            data,
            time: midi.time,
        }
    }
}

impl std::fmt::Debug for MidiCopy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Midi {{ time: {}, len: {}, data: {:?} }}",
            self.time,
            self.len,
            &self.data[..self.len]
        )
    }
}

fn main() {
    // open client
    let (client, _status) =
        jack::Client::new("midi_sine", jack::ClientOptions::NO_START_SERVER).unwrap();

    // create a sync channel to send back copies of midi messages we get
    let (sender, receiver) = sync_channel(64);

    // process logic
    let mut midi_output = client
        .register_port("midi_output", jack::MidiOut::default())
        .unwrap();
    let midi_input = client
        .register_port("midi_input", jack::MidiIn::default())
        .unwrap();
    let mut audio_output = client
        .register_port("audio_output", jack::AudioOut::default())
        .unwrap();

    // 3. define process callback handler
    let mut frequency = 220.0;
    let mut amplitude = 0.0;
    let sample_rate = client.sample_rate();
    let frame_t = 1.0 / sample_rate as f64;
    let mut time = 0.0;
    let (tx, rx) = bounded(1_000_000);

    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            let show_p = midi_input.iter(ps);
            for e in show_p {
                let c: MidiCopy = e.into();
                let _ = sender.try_send(c);

                if c.data[0] == 144 {
                    amplitude = 0.5;
                    frequency = (2.0_f64).powf((c.data[1] as f64-69.0)/12.0) * 440.0;
                } else if c.data[0] == 128 {
                    amplitude = 0.0;
                }

            }
            let mut put_p = midi_output.writer(ps);
            put_p
                .write(&jack::RawMidi {
                    time: 0,
                    bytes: &[
                        0b10010000, /* Note On, channel 1 */
                        0b01000000, /* Key number */
                        0b01111111, /* Velocity */
                    ],
                })
                .unwrap();
            put_p
                .write(&jack::RawMidi {
                    time: ps.n_frames() / 2,
                    bytes: &[
                        0b10000000, /* Note Off, channel 1 */
                        0b01000000, /* Key number */
                        0b01111111, /* Velocity */
                    ],
                })
                .unwrap();

            // Get output buffer
            let out = audio_output.as_mut_slice(ps);

            // Check frequency requests
            while let Ok(f) = rx.try_recv() {
                time = 0.0;
                frequency = f;
            }

            // Write output
            for v in out.iter_mut() {
                let x = frequency * time * 2.0 * std::f64::consts::PI;
                let y = amplitude * x.sin();
                *v = y as f32;
                time += frame_t;
            }

            // Continue as normal
            jack::Control::Continue
        },
    );

    // 4. Activate the client and connect the ports.
    let active_client = client.activate_async((), process).unwrap();
    active_client
        .as_client()
        .connect_ports_by_name("system:midi_capture_13", "midi_sine:midi_input")
        .unwrap();
    active_client
        .as_client()
        .connect_ports_by_name("midi_sine:audio_output", "system:playback_1")
        .unwrap();
    active_client
        .as_client()
        .connect_ports_by_name("midi_sine:audio_output", "system:playback_2")
        .unwrap();

    // processing starts here

    // spawn a non-real-time thread that prints out the midi messages we get
    std::thread::spawn(move || {
        while let Ok(m) = receiver.recv() {
            println!("{:?}", m);
        }
    });

    // wait
    println!("Press return to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // optional deactivation
    active_client.deactivate().unwrap();
}
