use core::task::Poll;

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};

use crate::print;
use crate::println;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("Scancode queue is full");
        } else {
            WAKER.wake();
        }
    } else {
        println!("Scancode queue is not initialized");
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("Cannot initialize scancode queue, it can only be init once");
        ScancodeStream { _private: () }
    }
}

static WAKER: AtomicWaker = AtomicWaker::new();

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");

        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    pc_keyboard::DecodedKey::Unicode(char) => print!("{}", char),
                    pc_keyboard::DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
