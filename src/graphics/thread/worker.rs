use std::sync::mpsc::Sender;

use crate::graphics::ResizeOp;

use super::{Output, WindowId};

pub(super) enum Message {
    Exit,
    ResizeOutput(WindowId, Output, ResizeOp, Sender<Output>),
    DestroyOutput(Output),
}

/// Controler for the worker thread for handling swapchain resizing,
/// destruction, and other show tasks. This avoids blocking the render thread
/// for however many frames it takes for the operation to complete.
pub(super) struct Worker {
    joiner: Option<std::thread::JoinHandle<()>>,
    sender: Sender<Message>,
}

impl Worker {
    pub fn new(notifier: Sender<super::Message>) -> Self {
        let (work_sender, work_receiver) = std::sync::mpsc::channel();

        let joiner = std::thread::spawn(move || {
            for msg in work_receiver {
                match msg {
                    Message::Exit => break,
                    Message::ResizeOutput(id, mut output, op, reply) => {
                        #[cfg(feature = "profile")]
                        let _s = tracing_tracy::client::span!("Worker::resize_output");

                        output.resize(op);
                        reply.send(output).unwrap();
                        notifier.send(super::Message::WorkerDone(id)).unwrap();
                    }
                    Message::DestroyOutput(output) => {
                        #[cfg(feature = "profile")]
                        let _s = tracing_tracy::client::span!("Worker::destroy_output");

                        std::mem::drop(output);
                    }
                }
            }
        });

        Self {
            joiner: Some(joiner),
            sender: work_sender,
        }
    }

    pub fn resize_output(
        &mut self,
        id: WindowId,
        output: Output,
        op: ResizeOp,
        reply: Sender<Output>,
    ) {
        self.sender
            .send(Message::ResizeOutput(id, output, op, reply))
            .unwrap();
    }

    pub fn destroy_output(&mut self, output: Output) {
        self.sender.send(Message::DestroyOutput(output)).unwrap();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.send(Message::Exit).unwrap();
        self.joiner.take().unwrap().join().unwrap();
    }
}
