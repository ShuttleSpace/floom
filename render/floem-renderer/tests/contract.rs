use floem_renderer::{render_once, FrameInfo, Renderer};
use floem_scene::{Color, DamageRegion, Point, Rect, Scene, SceneNode, Size};
use std::cell::RefCell;
use std::rc::Rc;

/// Fake backend that records draw ops from a Scene — no widgets, no GPU.
struct FakeRenderer {
    ops: Rc<RefCell<Vec<&'static str>>>,
    rects: Rc<RefCell<Vec<Rect>>>,
}

impl Renderer for FakeRenderer {
    fn begin_frame(&mut self, _info: &FrameInfo) {
        self.ops.borrow_mut().push("begin");
    }

    fn render_scene(&mut self, scene: &Scene, _damage: &DamageRegion) {
        self.ops.borrow_mut().push("scene");
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect { bounds, .. } => {
                    self.ops.borrow_mut().push("rect");
                    self.rects.borrow_mut().push(*bounds);
                }
                SceneNode::Image { .. } => self.ops.borrow_mut().push("image"),
                SceneNode::Text { .. } => self.ops.borrow_mut().push("text"),
                SceneNode::Group { .. } => self.ops.borrow_mut().push("group"),
            }
        }
    }

    fn end_frame(&mut self) {
        self.ops.borrow_mut().push("end");
    }
}

#[test]
fn fake_renderer_consumes_scene_without_widgets() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(20.0, 10.0)),
        fill: Color::rgb(255, 0, 0),
        radius: 0.0,
    });
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(5.0, 5.0), Size::new(4.0, 4.0)),
        fill: Color::BLACK,
        radius: 0.0,
    });
    scene.finish_update();

    let ops = Rc::new(RefCell::new(Vec::new()));
    let rects = Rc::new(RefCell::new(Vec::new()));
    let mut fake = FakeRenderer {
        ops: ops.clone(),
        rects: rects.clone(),
    };

    render_once(&mut fake, &FrameInfo::software(100, 100, 1), &scene);

    assert_eq!(
        ops.borrow().as_slice(),
        &["begin", "scene", "rect", "rect", "end"]
    );
    assert_eq!(rects.borrow().len(), 2);
}

#[test]
fn renderer_trait_is_object_safe() {
    fn assert_obj_safe(_: &dyn Renderer) {}
    let ops = Rc::new(RefCell::new(Vec::new()));
    let rects = Rc::new(RefCell::new(Vec::new()));
    let fake = FakeRenderer { ops, rects };
    assert_obj_safe(&fake);
}
