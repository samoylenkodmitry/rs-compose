//! Text widget implementation

#![allow(non_snake_case)]

use super::nodes::{ButtonNode, LayoutNode, TextNode};
use crate::composable;
use crate::modifier::Modifier;
use compose_core::{bubble_layout_dirty_in_composer, MutableState, Node, NodeId, State};
use std::rc::Rc;

#[derive(Clone)]
pub struct DynamicTextSource(Rc<dyn Fn() -> String>);

impl DynamicTextSource {
    pub fn new<F>(resolver: F) -> Self
    where
        F: Fn() -> String + 'static,
    {
        Self(Rc::new(resolver))
    }

    fn resolve(&self) -> String {
        (self.0)()
    }
}

impl PartialEq for DynamicTextSource {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for DynamicTextSource {}

#[derive(Clone, PartialEq, Eq)]
enum TextSource {
    Static(String),
    Dynamic(DynamicTextSource),
}

impl TextSource {
    fn resolve(&self) -> String {
        match self {
            TextSource::Static(text) => text.clone(),
            TextSource::Dynamic(dynamic) => dynamic.resolve(),
        }
    }
}

trait IntoTextSource {
    fn into_text_source(self) -> TextSource;
}

impl IntoTextSource for String {
    fn into_text_source(self) -> TextSource {
        TextSource::Static(self)
    }
}

impl<'a> IntoTextSource for &'a str {
    fn into_text_source(self) -> TextSource {
        TextSource::Static(self.to_string())
    }
}

impl<T> IntoTextSource for State<T>
where
    T: ToString + Clone + 'static,
{
    fn into_text_source(self) -> TextSource {
        let state = self.clone();
        TextSource::Dynamic(DynamicTextSource::new(move || state.value().to_string()))
    }
}

impl<T> IntoTextSource for MutableState<T>
where
    T: ToString + Clone + 'static,
{
    fn into_text_source(self) -> TextSource {
        let state = self.clone();
        TextSource::Dynamic(DynamicTextSource::new(move || state.value().to_string()))
    }
}

impl<F> IntoTextSource for F
where
    F: Fn() -> String + 'static,
{
    fn into_text_source(self) -> TextSource {
        TextSource::Dynamic(DynamicTextSource::new(self))
    }
}

impl IntoTextSource for DynamicTextSource {
    fn into_text_source(self) -> TextSource {
        TextSource::Dynamic(self)
    }
}

#[composable]
pub fn Text<S>(value: S, modifier: Modifier) -> NodeId
where
    S: IntoTextSource + Clone + PartialEq + 'static,
{
    let current = value.into_text_source().resolve();
    let id = compose_core::with_current_composer(|composer| {
        composer.emit_node(|| {
            let mut node = TextNode::default();
            node.modifier = modifier.clone();
            node.text = current.clone();
            node
        })
    });
    let mut needs_layout = false;
    let mut parent_to_invalidate = None;
    if let Err(err) = compose_core::with_node_mut(id, |node: &mut TextNode| {
        if node.text != current {
            node.text = current.clone();
            parent_to_invalidate = node.parent();
            needs_layout = true;
        }
        node.modifier = modifier.clone();
    }) {
        debug_assert!(false, "failed to update Text node: {err}");
    }
    if needs_layout {
        bubble_layout_dirty_for_text(parent_to_invalidate);
    }
    id
}

// DynamicTextSource is already public above

fn bubble_layout_dirty_for_text(mut parent: Option<NodeId>) {
    while let Some(node_id) = parent {
        if compose_core::with_node_mut(node_id, |_: &mut LayoutNode| ()).is_ok() {
            bubble_layout_dirty_in_composer::<LayoutNode>(node_id);
            return;
        }

        match compose_core::with_node_mut(node_id, |node: &mut ButtonNode| node.parent()) {
            Ok(next_parent) => parent = next_parent,
            Err(_) => break,
        }
    }

    // Fall back to a full render pass if we couldn't find a layout ancestor.
    crate::request_render_invalidation();
}
