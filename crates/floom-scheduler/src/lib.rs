//! Priority-aware frame scheduler with coalescing and budgeted drains.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

/// Scheduling priority. Lower discriminant runs earlier within a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Priority {
    Input = 0,
    State = 1,
    Animation = 2,
    Layout = 3,
    Paint = 4,
    Accessibility = 5,
    /// Idle work yields to all frame-critical priorities.
    Idle = 6,
}

impl Priority {
    pub const ALL: [Priority; 7] = [
        Priority::Input,
        Priority::State,
        Priority::Animation,
        Priority::Layout,
        Priority::Paint,
        Priority::Accessibility,
        Priority::Idle,
    ];

    pub fn is_frame_critical(self) -> bool {
        self != Priority::Idle
    }
}

/// Opaque task identity used for coalescing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

/// Work item invoked when the scheduler drains a queue.
pub type TaskFn = Box<dyn FnMut() + 'static>;

struct QueuedTask {
    id: TaskId,
    run: TaskFn,
}

/// Coalescing priority queues drained by [`Scheduler::tick`].
#[derive(Default)]
pub struct Scheduler {
    queues: [VecDeque<QueuedTask>; 7],
    /// TaskId → priority currently queued (for coalescing).
    pending: HashMap<TaskId, Priority>,
    next_id: u64,
    /// Wall-clock budget for a single [`tick`] drain of critical work.
    frame_budget: Duration,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            frame_budget: Duration::from_millis(8),
            ..Self::default()
        }
    }

    pub fn with_frame_budget(mut self, budget: Duration) -> Self {
        self.frame_budget = budget;
        self
    }

    pub fn set_frame_budget(&mut self, budget: Duration) {
        self.frame_budget = budget;
    }

    pub fn frame_budget(&self) -> Duration {
        self.frame_budget
    }

    /// Allocate a fresh task id.
    pub fn alloc_task_id(&mut self) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Schedule `run` at `priority`. If `id` is already queued, replace the
    /// payload and keep a single slot (coalesce). If the existing priority
    /// differs, the task is moved to the new priority.
    pub fn schedule(&mut self, priority: Priority, id: TaskId, run: TaskFn) {
        if let Some(old) = self.pending.insert(id, priority) {
            self.remove_from_queue(old, id);
        }
        self.queues[priority as usize].push_back(QueuedTask { id, run });
    }

    /// Convenience: allocate an id and schedule in one step.
    pub fn schedule_new(&mut self, priority: Priority, run: TaskFn) -> TaskId {
        let id = self.alloc_task_id();
        self.schedule(priority, id, run);
        id
    }

    pub fn pending_count(&self, priority: Priority) -> usize {
        self.queues[priority as usize].len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn remove_from_queue(&mut self, priority: Priority, id: TaskId) {
        let q = &mut self.queues[priority as usize];
        if let Some(pos) = q.iter().position(|t| t.id == id) {
            q.remove(pos);
        }
    }

    /// Drain frame-critical queues first (Input…Accessibility), respecting
    /// `frame_budget`. Idle tasks run only after critical queues are empty and
    /// remaining budget allows.
    ///
    /// Returns how many tasks ran.
    pub fn tick(&mut self, now: Instant) -> usize {
        let deadline = now + self.frame_budget;
        let mut ran = 0usize;

        for priority in Priority::ALL {
            if priority == Priority::Idle {
                continue;
            }
            ran += self.drain_priority(priority, Some(deadline));
            if Instant::now() >= deadline {
                return ran;
            }
        }

        // Idle only if no critical work remains.
        if self.has_critical_work() {
            return ran;
        }
        ran += self.drain_priority(Priority::Idle, Some(deadline));
        ran
    }

    /// Drain all pending work regardless of budget (tests / shutdown).
    pub fn flush(&mut self) -> usize {
        let mut ran = 0;
        for priority in Priority::ALL {
            ran += self.drain_priority(priority, None);
        }
        ran
    }

    fn has_critical_work(&self) -> bool {
        Priority::ALL
            .iter()
            .copied()
            .filter(|p| p.is_frame_critical())
            .any(|p| !self.queues[p as usize].is_empty())
    }

    fn drain_priority(&mut self, priority: Priority, deadline: Option<Instant>) -> usize {
        let mut ran = 0;
        while let Some(mut task) = self.queues[priority as usize].pop_front() {
            self.pending.remove(&task.id);
            (task.run)();
            ran += 1;
            if let Some(deadline) = deadline
                && Instant::now() >= deadline
            {
                break;
            }
        }
        ran
    }
}

/// Snapshot helper for tests: priorities that still have queued work.
pub fn pending_priorities(scheduler: &Scheduler) -> HashSet<Priority> {
    Priority::ALL
        .iter()
        .copied()
        .filter(|p| scheduler.pending_count(*p) > 0)
        .collect()
}
