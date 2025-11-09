use std::fmt;
use std::rc::Rc;

use compose_foundation::{ModifierNode, ModifierNodeChain, PointerEvent, PointerEventKind};

use crate::draw::DrawCommand;
use crate::modifier::Modifier;
use crate::modifier_nodes::{
    ClickableNode, ClipToBoundsNode, DrawCommandNode, PointerEventHandlerNode,
};

use super::{ModifierChainHandle, Point};

/// Snapshot of modifier node slices that impact draw and pointer subsystems.
#[derive(Clone, Default)]
pub struct ModifierNodeSlices {
    draw_commands: Vec<DrawCommand>,
    pointer_inputs: Vec<Rc<dyn Fn(PointerEvent)>>,
    click_handlers: Vec<Rc<dyn Fn(Point)>>,
    clip_to_bounds: bool,
}

impl ModifierNodeSlices {
    pub fn draw_commands(&self) -> &[DrawCommand] {
        &self.draw_commands
    }

    pub fn pointer_inputs(&self) -> &[Rc<dyn Fn(PointerEvent)>] {
        &self.pointer_inputs
    }

    pub fn click_handlers(&self) -> &[Rc<dyn Fn(Point)>] {
        &self.click_handlers
    }

    pub fn clip_to_bounds(&self) -> bool {
        self.clip_to_bounds
    }

    fn extend(&mut self, other: ModifierNodeSlices) {
        self.draw_commands.extend(other.draw_commands);
        self.pointer_inputs.extend(other.pointer_inputs.into_iter());
        self.click_handlers.extend(other.click_handlers.into_iter());
        self.clip_to_bounds |= other.clip_to_bounds;
    }
}

impl fmt::Debug for ModifierNodeSlices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifierNodeSlices")
            .field("draw_commands", &self.draw_commands.len())
            .field("pointer_inputs", &self.pointer_inputs.len())
            .field("click_handlers", &self.click_handlers.len())
            .field("clip_to_bounds", &self.clip_to_bounds)
            .finish()
    }
}

/// Collects modifier node slices directly from a reconciled [`ModifierNodeChain`].
pub fn collect_modifier_slices(chain: &ModifierNodeChain) -> ModifierNodeSlices {
    let mut slices = ModifierNodeSlices::default();

    for node in chain.pointer_input_nodes() {
        let modifier_node = node as &dyn ModifierNode;
        if let Some(clickable) = modifier_node.as_any().downcast_ref::<ClickableNode>() {
            let handler = clickable.handler();
            slices.click_handlers.push(handler.clone());
            let pointer_handler = handler.clone();
            slices
                .pointer_inputs
                .push(Rc::new(move |event: PointerEvent| {
                    if matches!(event.kind, PointerEventKind::Down) {
                        pointer_handler(Point {
                            x: event.position.x,
                            y: event.position.y,
                        });
                    }
                }));
        } else if let Some(handler) = modifier_node
            .as_any()
            .downcast_ref::<PointerEventHandlerNode>()
        {
            slices.pointer_inputs.push(handler.handler());
        }
    }

    for node in chain.draw_nodes() {
        let modifier_node = node as &dyn ModifierNode;
        if let Some(commands) = modifier_node.as_any().downcast_ref::<DrawCommandNode>() {
            slices
                .draw_commands
                .extend(commands.commands().iter().cloned());
        }
        if modifier_node.as_any().is::<ClipToBoundsNode>() {
            slices.clip_to_bounds = true;
        }
    }

    slices
}

/// Collects modifier node slices by instantiating a temporary node chain from a [`Modifier`].
pub fn collect_slices_from_modifier(modifier: &Modifier) -> ModifierNodeSlices {
    let mut handle = ModifierChainHandle::new();
    handle.update(modifier);
    collect_modifier_slices(handle.chain())
}
