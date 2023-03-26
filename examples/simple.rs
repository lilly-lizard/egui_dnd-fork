use eframe::egui::{Context, Id};
use eframe::{egui, App, Frame, NativeOptions};

use egui_dnd::utils::shift_vec;
use egui_dnd::{DragDropResponse, DragDropUi, DragableItem};

struct DnDApp {
    // DragDropUi stores state about the currently dragged item
    dnd: DragDropUi,
    items: Vec<ItemType>,
}

impl Default for DnDApp {
    fn default() -> Self {
        DnDApp {
            dnd: DragDropUi::default(),
            items: ["alfred", "bernhard", "christian"]
                .iter()
                .map(|name| ItemType {
                    name: name.to_string(),
                })
                .collect(),
        }
    }
}

struct ItemType {
    name: String,
}

// We need this to uniquely identify items. You can also implement the Hash trait.
impl DragableItem for ItemType {
    fn id(&self) -> Id {
        Id::new(&self.name)
    }
}

impl App for DnDApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let response =
                // make sure this is called in a vertical layout.
                // Horizontal sorting is not supported yet.
                self.dnd.ui::<ItemType>(ui, self.items.iter_mut(), |ui, handle, index, item| {
                    ui.horizontal(|ui| {
                        // Anything in the handle can be used to drag the item
                        handle.ui(ui, item, |ui| {
                            ui.label(format!("{} grab", index));
                        });

                        ui.label(&item.name);
                    });
                });

            // After the drag is complete, we get a response containing the old index of the
            // dragged item, as well as the index it was moved to. You can use the
            // shift_vec function as a helper if you store your items in a Vec.
            if let DragDropResponse::Completed(drag_indices) = response {
                shift_vec(drag_indices.source, drag_indices.target, &mut self.items);
            }
        });
    }
}

pub fn main() {
    eframe::run_native(
        "DnD Example",
        NativeOptions::default(),
        Box::new(|_a| Box::new(DnDApp::default())),
    )
    .unwrap();
}
