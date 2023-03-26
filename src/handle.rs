use crate::{DragDropUi, DragableItem};
use egui::{self, CursorIcon, Pos2, Sense, Ui};

/// [Handle::ui] is used to draw the drag handle
pub struct Handle<'a> {
    pub state: &'a mut DragDropUi,
    pub placeholder: bool,
}

/// The part of the item ui thats draggable. Accessible by the user with the `item_ui` parameter of [`DragDropUi::ui`]
impl<'a> Handle<'a> {
    pub fn ui<T: DragableItem>(self, ui: &mut Ui, item: &T, contents: impl FnOnce(&mut Ui)) {
        if self.placeholder {
            // if this is meant to be a placeholder ui, dont do the draggable stuff.
            contents(ui);
            return;
        }

        // add contents to ui
        let added_contents = ui.scope(contents);
        let dragable_response = ui.interact(added_contents.response.rect, item.id(), Sense::drag());

        // if pointer hovering above this widget, update pointer icon
        if dragable_response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }

        // if dragging this widget just began, store the intial pointer position relative to the widget origin
        if dragable_response.drag_started() {
            let top_left = added_contents.response.rect.min.to_vec2();
            let pointer_pos = dragable_response
                .interact_pointer_pos()
                .unwrap_or(Pos2::default())
                .to_vec2();
            self.state.drag_delta = Some(top_left - pointer_pos);
        }
    }
}
