use floem_legacy_reactive::{Effect, SignalGet};
use floem_store::store;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq, Debug)]
struct AppState {
    name: String,
    count: u32,
}

#[test]
fn unrelated_selector_not_invalidated() {
    let s = store(AppState {
        name: "a".into(),
        count: 0,
    });

    let name = s.select(|st| st.name.clone());
    let count = s.select(|st| st.count);

    let name_runs = Rc::new(Cell::new(0u32));
    let count_runs = Rc::new(Cell::new(0u32));

    {
        let name_runs = name_runs.clone();
        Effect::new(move |_| {
            let _ = name.get();
            name_runs.set(name_runs.get() + 1);
        });
    }
    {
        let count_runs = count_runs.clone();
        Effect::new(move |_| {
            let _ = count.get();
            count_runs.set(count_runs.get() + 1);
        });
    }

    assert_eq!(name_runs.get(), 1);
    assert_eq!(count_runs.get(), 1);

    s.update(|st| st.count += 1);
    assert_eq!(count.get(), 1);
    assert_eq!(count_runs.get(), 2, "count selector must re-run");
    assert_eq!(
        name_runs.get(),
        1,
        "name selector must not re-run when only count changes"
    );

    s.update(|st| st.name = "b".into());
    assert_eq!(name.get(), "b");
    assert_eq!(name_runs.get(), 2);
    assert_eq!(
        count_runs.get(),
        2,
        "count selector must not re-run when only name changes"
    );
}
