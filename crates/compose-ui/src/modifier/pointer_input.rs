use super::{Modifier, PointerEvent};
use crate::modifier_nodes::PointerEventHandlerElement;
use std::rc::Rc;

impl Modifier {
    pub fn pointer_input(handler: impl Fn(PointerEvent) + 'static) -> Self {
        let handler = Rc::new(handler);
        Self::with_element(PointerEventHandlerElement::new(handler), |_| {})
    }
}
