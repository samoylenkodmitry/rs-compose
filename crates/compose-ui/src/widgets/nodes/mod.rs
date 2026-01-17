use compose_core::{Node, NodeId};

mod layout_node;

pub use layout_node::IntrinsicKind;
pub use layout_node::LayoutNode;
pub(crate) use layout_node::LayoutNodeCacheHandles;
pub(crate) use layout_node::{allocate_virtual_node_id, is_virtual_node, register_layout_node};

pub fn compose_node<N: Node + 'static>(init: impl FnOnce() -> N) -> NodeId {
    compose_core::with_current_composer(|composer| composer.emit_node(init))
}
