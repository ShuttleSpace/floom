use floem_scene::{Resource, ResourceKind, ResourceRegistry};

#[test]
fn retain_release_refcount() {
    let mut reg = ResourceRegistry::new();
    let id = reg.insert(Resource {
        kind: ResourceKind::Blob { bytes: 100 },
    });
    assert_eq!(reg.refs(id), Some(1));
    assert!(reg.retain(id));
    assert_eq!(reg.refs(id), Some(2));
    assert!(reg.release(id));
    assert_eq!(reg.refs(id), Some(1));
    assert!(reg.release(id));
    assert!(reg.get(id).is_none());
}

#[test]
fn shared_resource_survives_one_release() {
    let mut reg = ResourceRegistry::new();
    let id = reg.insert(Resource {
        kind: ResourceKind::Image {
            width: 10,
            height: 10,
        },
    });
    reg.retain(id);
    reg.release(id);
    assert!(reg.get(id).is_some());
    reg.release(id);
    assert!(reg.get(id).is_none());
}

#[test]
fn budget_evicts_single_ref_resources() {
    let mut reg = ResourceRegistry::new().with_budget(150);
    let a = reg.insert(Resource {
        kind: ResourceKind::Blob { bytes: 100 },
    });
    let b = reg.insert(Resource {
        kind: ResourceKind::Blob { bytes: 100 },
    });
    // Inserting b should have evicted a (refs==1).
    assert!(reg.get(a).is_none());
    assert!(reg.get(b).is_some());
    assert!(reg.eviction_count() >= 1);
}
