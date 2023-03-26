use core::task::Waker;

use alloc::{collections::BTreeMap, sync::Arc};
use crossbeam_queue::ArrayQueue;

use super::{Task, TaskId};

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("Already exists");
        }
        self.task_queue.push(id).expect("Queue full");
    }

    fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Ok(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };

            //let waker = waker_cache.entry(task_id).or_insert_with(|| TaskWa);
        }
    }
}
