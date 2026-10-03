use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{dyn_view, Empty, Label, Stack};
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_widget_md_icon::MdIcon;
use floom_core_theme::props::treeview::{TreeviewNode, TreeviewProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdTreeviewStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub indent: f32,
    pub row_height: f32,
    pub icon_size: f32,
    pub expander_size: f32,
    pub font_size: f32,
}

pub struct MdTreeview {
    props: TreeviewProps,
    theme: RwSignal<MultiTheme>,
    nodes: RwSignal<Vec<TreeviewNode>>,
}

fn set_expanded(tree: &mut Vec<TreeviewNode>, path: &[usize]) {
    if path.is_empty() {
        return;
    }
    let mut node_ref = tree;
    for &idx in &path[..path.len() - 1] {
        node_ref = &mut node_ref[idx].children;
    }
    let last = *path.last().unwrap();
    node_ref[last].expanded = !node_ref[last].expanded;
}

fn flatten(
    nodes: &[TreeviewNode],
    out: &mut Vec<AnyView>,
    path: &[usize],
    theme: RwSignal<MultiTheme>,
    indent: f32,
    row_height: f32,
    icon_size: f32,
    expander_size: f32,
    font_size: f32,
    on_node_click: &Rc<Option<Box<dyn Fn(String)>>>,
    toggle: &Rc<dyn Fn(Vec<usize>)>,
) {
    for (i, node) in nodes.iter().enumerate() {
        let mut p = path.to_vec();
        p.push(i);
        let depth = p.len();
        let has_children = !node.children.is_empty();
        let label = node.label.clone();
        let icon = node.icon.clone();
        let expanded = node.expanded;
        let on_node_click = on_node_click.clone();
        let toggle = toggle.clone();
        let path_clone = p.clone();

        let expander = if has_children {
            let t2 = toggle.clone();
            let icon_name = if expanded { "expand_more" } else { "chevron_right" };
            MdIcon::new(icon_name)
                .size(expander_size)
                .view()
                .style(move |s| {
                    s.width(expander_size)
                        .height(expander_size)
                        .items_center()
                        .justify_center()
                        .cursor(CursorStyle::Pointer)
                })
                .on_event(listener::Click, move |_, _| {
                    let pc = path_clone.clone();
                    t2(pc);
                    EventPropagation::Continue
                })
                .into_any()
        } else {
            Empty::new()
                .style(move |s| s.width(expander_size).height(expander_size))
                .into_any()
        };

        let icon_view = if let Some(ic) = icon {
            MdIcon::new(ic)
                .size(icon_size * 0.7)
                .view()
                .style(move |s| {
                    s.width(icon_size)
                        .height(icon_size)
                        .items_center()
                        .justify_center()
                })
                .into_any()
        } else {
            Empty::new().style(move |s| s.width(icon_size)).into_any()
        };

        let indent_sp = Empty::new()
            .style(move |s| s.width((depth as f32 - 1.0) * indent))
            .into_any();

        let on_node_click_label = on_node_click.clone();
        let label_view = Label::new(label.clone())
            .style(move |s| {
                s.flex_grow(1.0)
                    .color(theme.get().colors().on_surface.to_peniko())
                    .font_size(font_size)
                    .items_center()
            })
            .on_event(listener::Click, move |_, _| {
                if let Some(cb) = on_node_click_label.as_ref() {
                    cb(label.clone());
                }
                EventPropagation::Continue
            })
            .into_any();

        let row = Stack::horizontal((indent_sp, expander, icon_view, label_view))
            .style(move |s| {
                s.height(row_height)
                    .items_center()
                    .gap(4.0)
                    .padding_horiz(8.0)
                    .cursor(CursorStyle::Pointer)
                    .hover(move |s| {
                        s.background(with_alpha(theme.get().colors().on_surface, 8).to_peniko())
                    })
            });
        out.push(row.into_any());

        if expanded && has_children {
            flatten(
                &node.children,
                out,
                &p,
                theme,
                indent,
                row_height,
                icon_size,
                expander_size,
                font_size,
                &on_node_click,
                &toggle,
            );
        }
    }
}

impl MdTreeview {
    pub fn new() -> Self {
        let props = TreeviewProps::default();
        Self {
            theme: theme_signal(),
            nodes: RwSignal::new(props.nodes.clone()),
            props,
        }
    }

    pub fn nodes(mut self, nodes: Vec<TreeviewNode>) -> Self {
        self.props.nodes = nodes.clone();
        self.nodes.set(nodes);
        self
    }

    pub fn on_node_click(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.props.on_node_click = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> MdTreeviewStyle {
        let theme = self.theme.get();
        let ComponentTokens::Treeview(tokens) = theme.component(ComponentId::Treeview) else {
            unreachable!("ComponentId::Treeview must resolve to TreeviewTokens")
        };
        MdTreeviewStyle {
            design_system: "material",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            indent: tokens.indent,
            row_height: tokens.row_height,
            icon_size: tokens.icon_size,
            expander_size: tokens.expander_size,
            font_size: tokens.font_size,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let nodes = self.nodes;
        let on_node_click: Rc<Option<Box<dyn Fn(String)>>> = Rc::new(self.props.on_node_click);

        let tokens = match theme.get().component(ComponentId::Treeview) {
            ComponentTokens::Treeview(t) => t,
            _ => unreachable!(),
        };
        let indent = tokens.indent;
        let row_height = tokens.row_height;
        let icon_size = tokens.icon_size;
        let expander_size = tokens.expander_size;
        let font_size = tokens.font_size;

        let toggle: Rc<dyn Fn(Vec<usize>)> = Rc::new(move |path: Vec<usize>| {
            nodes.update(|tree| set_expanded(tree, &path));
        });

        let area = dyn_view(move || {
            let tree = nodes.get();
            let on_node_click = on_node_click.clone();
            let toggle = toggle.clone();
            let mut rows: Vec<AnyView> = Vec::new();
            flatten(
                &tree,
                &mut rows,
                &[],
                theme,
                indent,
                row_height,
                icon_size,
                expander_size,
                font_size,
                &on_node_click,
                &toggle,
            );
            list(rows).style(|s| s.gap(0.0))
        });

        area.style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko())
        })
    }
}

impl Default for MdTreeview {
    fn default() -> Self {
        Self::new()
    }
}
