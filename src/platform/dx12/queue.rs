use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Graphics::Direct3D12::{
        ID3D12CommandList, ID3D12CommandQueue, ID3D12Device, ID3D12Fence, D3D12_COMMAND_LIST_TYPE,
        D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_FENCE_FLAG_NONE,
    },
    System::Threading::{CreateEventW, WaitForSingleObject, INFINITE},
};

use crate::platform::gfx::SubmitId;

/// A queue of GPU commands.
///
/// Based on the implementation described here: <https://alextardif.com/D3D11To12P1.html>
pub struct Queue {
    pub queue: ID3D12CommandQueue,
    fence: ID3D12Fence,
    fence_event: Mutex<HANDLE>,
    num_submitted: AtomicU64,
    num_completed: AtomicU64,
}

impl Queue {
    pub fn new(device: &ID3D12Device, kind: D3D12_COMMAND_LIST_TYPE) -> Self {
        let queue = unsafe {
            device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type: kind,
                Priority: 0,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            })
        }
        .unwrap();

        let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }.unwrap();
        let fence_event = unsafe { CreateEventW(None, false, false, None) }.unwrap();

        unsafe { fence.Signal(0) }.unwrap();

        Self {
            queue,
            fence,
            fence_event: Mutex::new(fence_event),
            num_submitted: AtomicU64::new(0),
            num_completed: AtomicU64::new(0),
        }
    }

    /// Causes the CPU to wait until the given submission has completed.
    pub fn wait(&self, submission: SubmitId) {
        if self.is_done(submission) {
            return;
        }

        {
            // TODO: this would be faster if we could use an event per thread.

            let event = {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("wait for lock");

                self.fence_event.lock()
            };

            unsafe {
                self.fence
                    .SetEventOnCompletion(submission.0, *event)
                    .expect("out of memory");
            }

            unsafe {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("wait for fence event");

                WaitForSingleObject(*event, INFINITE);
            }
        }

        let _ = self
            .num_completed
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                (old < submission.0).then_some(submission.0)
            });
    }

    /// Causes the CPU to wait until all submissions have completed.
    pub fn wait_idle(&self) {
        // We have to increment the fence value before waiting, because DXGI may
        // submit work to the queue on our behalf when we call `Present`.
        // Without this, we end up stomping over the currently presenting frame
        // when resizing or destroying the swapchain.
        let id = {
            // todo: relax ordering if possible
            let signal = self.num_submitted.fetch_add(1, Ordering::SeqCst);
            unsafe { self.queue.Signal(&self.fence, signal) }.unwrap();
            SubmitId(signal)
        };

        self.wait(id);
    }

    pub fn is_done(&self, submission: SubmitId) -> bool {
        if submission.0 > self.num_completed.load(Ordering::Acquire) {
            self.poll_fence();
        }

        submission.0 <= self.num_completed.load(Ordering::Acquire)
    }

    #[tracing::instrument(skip(self))]
    pub fn submit(&self, commands: &ID3D12CommandList) -> SubmitId {
        // todo: relax ordering if possible
        let signal = self.num_submitted.fetch_add(1, Ordering::SeqCst);

        unsafe { self.queue.ExecuteCommandLists(&[Some(commands.clone())]) };
        unsafe { self.queue.Signal(&self.fence, signal) }.unwrap();

        SubmitId(signal)
    }

    fn poll_fence(&self) {
        let fence_value = unsafe { self.fence.GetCompletedValue() };

        let _ = self
            .num_completed
            // Don't know what ordering to use here, so just use SeqCst for both
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                (old < fence_value).then_some(fence_value)
            });
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        self.wait_idle();

        let event = self.fence_event.lock();
        unsafe { CloseHandle(*event) }.unwrap();
    }
}
