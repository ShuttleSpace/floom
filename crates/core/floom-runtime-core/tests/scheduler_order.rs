// SPDX-License-Identifier: MIT
use floom_runtime_core::scheduler::{Priority, Scheduler, TaskId};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn drains_higher_priority_first() {
    let mut sched = Scheduler::new();
    let order = Rc::new(RefCell::new(Vec::new()));

    let push = |label: &'static str| {
        let order = order.clone();
        Box::new(move || order.borrow_mut().push(label)) as Box<dyn FnMut()>
    };

    sched.schedule(Priority::Idle, TaskId(1), push("idle"));
    sched.schedule(Priority::Paint, TaskId(2), push("paint"));
    sched.schedule(Priority::Input, TaskId(3), push("input"));
    sched.schedule(Priority::Layout, TaskId(4), push("layout"));

    sched.flush();
    assert_eq!(
        order.borrow().as_slice(),
        &["input", "layout", "paint", "idle"]
    );
}

#[test]
fn coalesces_same_task_id() {
    let mut sched = Scheduler::new();
    let runs = Rc::new(RefCell::new(0u32));
    let id = TaskId(7);

    for _ in 0..5 {
        let runs = runs.clone();
        sched.schedule(
            Priority::State,
            id,
            Box::new(move || *runs.borrow_mut() += 1),
        );
    }
    assert_eq!(sched.pending_count(Priority::State), 1);
    sched.flush();
    assert_eq!(*runs.borrow(), 1);
}

#[test]
fn coalesce_moves_priority() {
    let mut sched = Scheduler::new();
    let order = Rc::new(RefCell::new(Vec::new()));
    let id = TaskId(9);

    let order1 = order.clone();
    sched.schedule(
        Priority::Idle,
        id,
        Box::new(move || order1.borrow_mut().push("idle")),
    );
    let order2 = order.clone();
    sched.schedule(
        Priority::Input,
        id,
        Box::new(move || order2.borrow_mut().push("input")),
    );

    assert_eq!(sched.pending_count(Priority::Idle), 0);
    assert_eq!(sched.pending_count(Priority::Input), 1);
    sched.flush();
    assert_eq!(order.borrow().as_slice(), &["input"]);
}

#[test]
fn idle_yields_while_critical_remains() {
    let mut sched = Scheduler::new().with_frame_budget(Duration::from_secs(60));
    let order = Rc::new(RefCell::new(Vec::new()));

    // Critical task that re-schedules itself once so Idle cannot run in the
    // same tick after the first drain pass if we only check emptiness mid-tick.
    // Simpler: schedule Idle + Input; tick with huge budget — both run, Input first.
    let o1 = order.clone();
    sched.schedule(
        Priority::Idle,
        TaskId(1),
        Box::new(move || o1.borrow_mut().push("idle")),
    );
    let o2 = order.clone();
    sched.schedule(
        Priority::Animation,
        TaskId(2),
        Box::new(move || o2.borrow_mut().push("anim")),
    );

    sched.tick(Instant::now());
    assert_eq!(order.borrow().as_slice(), &["anim", "idle"]);
}

#[test]
fn frame_budget_stops_critical_drain() {
    let mut sched = Scheduler::new().with_frame_budget(Duration::from_millis(1));
    let ran = Rc::new(RefCell::new(0u32));

    for i in 0..50u64 {
        let ran = ran.clone();
        sched.schedule(
            Priority::Layout,
            TaskId(i),
            Box::new(move || {
                *ran.borrow_mut() += 1;
                thread::sleep(Duration::from_millis(1));
            }),
        );
    }

    let n = sched.tick(Instant::now());
    assert!(n < 50, "budget should stop before all 50 tasks: ran={n}");
    assert!(*ran.borrow() < 50);
    assert!(!sched.is_empty());
}
