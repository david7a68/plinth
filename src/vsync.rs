use std::{
    sync::mpsc::{channel, Sender},
    thread::JoinHandle,
};

use windows::Win32::Graphics::DirectComposition::DCompositionWaitForCompositorClock;

use crate::render;

enum Message {
    Start,
    Stop,
    Exit,
}

pub struct VSyncSource {
    sender: Sender<Message>,
    joiner: Option<JoinHandle<()>>,
}

impl VSyncSource {
    pub fn new(destination: Sender<render::Message>) -> Self {
        let (sender, receiver) = channel();

        let joiner = std::thread::spawn(move || loop {
            unsafe { DCompositionWaitForCompositorClock(None, u32::MAX) };

            {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("VSync");

                destination.send(render::Message::VSync).unwrap();
            }

            let mut start_stop = 0;
            let mut exit = false;

            while let Ok(message) = receiver.try_recv() {
                match message {
                    Message::Start => start_stop += 1,
                    Message::Stop => start_stop -= 1,
                    Message::Exit => exit = true,
                }
            }

            if exit {
                break;
            } else if start_stop < 0 {
                tracing::info!("Stop VSync clock");
                while let Ok(message) = receiver.recv() {
                    match message {
                        Message::Start => {
                            tracing::info!("Resume VSync clock");
                            break;
                        }
                        Message::Exit => return,
                        _ => (),
                    }
                }
            }
        });

        Self {
            sender,
            joiner: Some(joiner),
        }
    }

    pub fn start(&self) {
        self.sender.send(Message::Start).unwrap();
    }

    pub fn stop(&self) {
        self.sender.send(Message::Stop).unwrap();
    }
}

impl Drop for VSyncSource {
    fn drop(&mut self) {
        self.sender.send(Message::Exit).unwrap();
        self.joiner.take().unwrap().join().unwrap();
    }
}
