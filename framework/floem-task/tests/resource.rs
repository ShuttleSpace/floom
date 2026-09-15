use floem_reactive::Scope;
use floem_task::{CancelToken, ResourceState, SyncExecutor, resource, spawn, ui_spawn};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn sync_executor_runs_immediately() {
    let ran = Rc::new(Cell::new(false));
    let flag = ran.clone();
    ui_spawn(&SyncExecutor, move || flag.set(true));
    assert!(ran.get());
}

#[test]
fn resource_ready_on_success() {
    let parent = Scope::new();
    let res = resource(parent, &SyncExecutor, |_cancel| Ok::<_, &str>(42));
    assert_eq!(res.get(), ResourceState::Ready(42));
}

#[test]
fn resource_error_on_failure() {
    let parent = Scope::new();
    let res = resource(parent, &SyncExecutor, |_cancel| Err::<i32, _>("boom"));
    assert_eq!(res.get(), ResourceState::Error("boom"));
}

#[test]
fn cancelled_loader_does_not_commit() {
    /// Executor that records tasks without running them until flushed.
    struct DeferredExecutor {
        queue: Rc<Cell<Vec<Box<dyn FnOnce()>>>>,
    }

    impl floem_task::TaskExecutor for DeferredExecutor {
        fn spawn_ui(&self, task: Box<dyn FnOnce()>) {
            let mut q = self.queue.take();
            q.push(task);
            self.queue.set(q);
        }
    }

    let queue = Rc::new(Cell::new(Vec::new()));
    let exec = DeferredExecutor {
        queue: queue.clone(),
    };
    let parent = Scope::new();
    let res = resource(parent, &exec, |_c| Ok::<_, &str>(7));
    assert_eq!(res.get(), ResourceState::Loading);

    res.cancel();
    // Flush deferred work after cancel — result must be ignored.
    let tasks = queue.take();
    for task in tasks {
        task();
    }
    assert_eq!(res.get(), ResourceState::Loading);
}

#[test]
fn scope_dispose_cancels_token_via_drop() {
    let parent = Scope::new();
    let seen = Arc::new(AtomicBool::new(false));
    let seen2 = seen.clone();
    let res = resource(parent, &SyncExecutor, move |cancel: CancelToken| {
        // Sync executor runs inline before Resource is returned; use a deferred
        // pattern instead — verify Drop cancels.
        let _ = cancel;
        seen2.store(true, Ordering::SeqCst);
        Ok::<_, &str>(1)
    });
    assert!(seen.load(Ordering::SeqCst));
    let token = res.cancel_token();
    drop(res);
    assert!(token.is_cancelled());
}

#[test]
fn spawn_alias_works() {
    let n = Rc::new(Cell::new(0));
    let n2 = n.clone();
    spawn(&SyncExecutor, move || n2.set(3));
    assert_eq!(n.get(), 3);
}
