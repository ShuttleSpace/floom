use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem_widget_cupertino_icon::cupertino_icon_svg;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::{svg, dyn_view, Empty, Label, Stack};
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::treeview::{TreeviewNode, TreeviewProps};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTreeviewStyle {
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

pub struct CupertinoTreeview {
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
            let chevron_name = if expanded { "expand_more" } else { "chevron_right" };
            svg(move || cupertino_icon_svg(chevron_name))
                .style(move |s| {
                    s.size(expander_size, expander_size)
                        .items_center()
                        .justify_center()
                        .color(theme.get().colors().on_surface_variant.to_peniko())
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
            Label::new(ic)
                .style(move |s| {
                    s.width(icon_size)
                        .height(icon_size)
                        .items_center()
                        .justify_center()
                        .font_size(icon_size * 0.7)
                        .color(theme.get().colors().on_surface_variant.to_peniko())
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

impl CupertinoTreeview {
    pub fn new() -> Self {
        let props = TreeviewProps::default();
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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

    pub fn resolved_style(&self) -> CupertinoTreeviewStyle {
        let theme = self.theme.get();
        let ComponentTokens::Treeview(tokens) = theme.component(ComponentId::Treeview) else {
            unreachable!("ComponentId::Treeview must resolve to TreeviewTokens")
        };
        CupertinoTreeviewStyle {
            design_system: "cupertino",
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

impl Default for CupertinoTreeview {
    fn default() -> Self {
        Self::new()
    }
}
