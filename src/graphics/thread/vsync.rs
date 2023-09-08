use std::{
    sync::mpsc::{channel, Sender},
    thread::JoinHandle,
};

use windows::Win32::Graphics::DirectComposition::DCompositionWaitForCompositorClock;

enum Message {
    Next,
    Exit,
}

pub(super) struct VSyncSource {
    sender: Sender<Message>,
    joiner: Option<JoinHandle<()>>,
}

impl VSyncSource {
    pub fn new(destination: Sender<super::Message>) -> Self {
        let (sender, receiver) = channel();

        let joiner = std::thread::spawn(move || {
            // todo: tracy name thread

            loop {
                for msg in &receiver {
                    match msg {
                        Message::Next => {
                            unsafe { DCompositionWaitForCompositorClock(None, u32::MAX) };

                            #[cfg(feature = "profile")]
                            let _s = tracing_tracy::client::span!("VSync");

                            destination.send(super::Message::VSync).unwrap();
                        }
                        Message::Exit => return,
                    }
                }

                {
                    #[cfg(feature = "profile")]
                    let _s = tracing_tracy::client::span!("VSync");

                    destination.send(super::Message::VSync).unwrap();
                }
            }
        });

        Self {
            sender,
            joiner: Some(joiner),
        }
    }

    pub fn next(&self) {
        self.sender.send(Message::Next).unwrap();
    }
}

impl Drop for VSyncSource {
    fn drop(&mut self) {
        self.sender.send(Message::Exit).unwrap();
        self.joiner.take().unwrap().join().unwrap();
    }
}
