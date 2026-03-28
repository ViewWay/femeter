//! Task scheduler for cooperative multitasking

pub struct TaskScheduler {
    tasks: [ScheduledTask; 16],
    count: usize,
}

#[derive(Clone, Copy)]
struct ScheduledTask {
    period_ms: u32,
    last_run: u64,
    active: bool,
}

impl TaskScheduler {
    pub const fn new() -> Self {
        Self {
            tasks: [ScheduledTask { period_ms: 0, last_run: 0, active: false }; 16],
            count: 0,
        }
    }

    pub fn register(&mut self, _id: usize, period_ms: u32) {
        if self.count < self.tasks.len() {
            self.tasks[self.count] = ScheduledTask {
                period_ms,
                last_run: 0,
                active: true,
            };
            self.count += 1;
        }
    }

    pub fn poll(&mut self, now_ms: u64) -> TaskIter<'_> {
        TaskIter { scheduler: self, index: 0, now_ms }
    }
}

pub struct TaskIter<'a> {
    scheduler: &'a mut TaskScheduler,
    index: usize,
    now_ms: u64,
}

impl<'a> Iterator for TaskIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        while self.index < self.scheduler.count {
            let idx = self.index;
            self.index += 1;
            let task = &mut self.scheduler.tasks[idx];
            if task.active && self.now_ms.wrapping_sub(task.last_run) >= task.period_ms as u64 {
                task.last_run = self.now_ms;
                return Some(idx);
            }
        }
        None
    }
}
