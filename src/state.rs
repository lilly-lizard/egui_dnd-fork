use egui::{self, CursorIcon, Id, LayerId, Order, Pos2, Rect, Sense, Shape, Ui, Vec2};
use std::hash::Hash;

use crate::utils::shift_vec;

pub trait DragDropItem {
    fn id(&self) -> Id;
}

impl<T: Hash> DragDropItem for T {
    fn id(&self) -> Id {
        Id::new(self)
    }
}

#[derive(Default, Clone)]
pub struct DragIndices {
    pub source: usize,
    pub target: usize,
}

/// DragDropStatus containing the potential list updates during and after a drag & drop event
/// `current_drag` will contain a [DragDropStatus] when something is being dragged right now and can be
/// used update some state while the drag is in progress.
/// `completed` contains a [DragDropStatus] after a successful drag & drop event. It should be used to
/// update positions of the affected items. If the source is a vec, [shift_vec] can be used.
pub enum DragDropResponse {
    NoDrag,
    CurrentDrag(DragIndices),
    Completed(DragIndices),
}

#[derive(Default, Clone)]
pub struct DragDropUi {
    drag_indices: Option<DragIndices>,
    /// Pointer position relative to the origin of the dragged widget when dragging began
    drag_delta: Option<Vec2>,
}

/// [Handle::ui] is used to draw the drag handle
pub struct Handle<'a> {
    state: &'a mut DragDropUi,
    placeholder: bool,
}

/// The part of the item ui thats draggable. Accessible by the user with the `item_ui` parameter of [`DragDropUi::ui`]
impl<'a> Handle<'a> {
    pub fn ui<T: DragDropItem>(self, ui: &mut Ui, item: &T, contents: impl FnOnce(&mut Ui)) {
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
            ui.output().cursor_icon = CursorIcon::Grab;
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

/// [DragDropUi] stores the state of the Drag & Drop list.
///
/// `item_ui` should be a function to draw the ui elements for each item in `items`. Its arguments are:
/// - a mutable reference to the ui
/// - a `Handle` that can be used to draw the draggable part of the item ui
/// - the index of the current item in the `items` list
/// - a reference to the current item in the `items` list
///
/// # Example
/// ```rust
/// use egui_dnd::DragDropUi;
/// use eframe::App;
/// use eframe::egui::Context;
/// use eframe::Frame;
/// use eframe::egui::CentralPanel;
/// use egui_dnd::utils::shift_vec;
///
/// struct DnDApp {
///     items: Vec<String>,
///     dnd: DragDropUi,
/// }
///
///
/// impl App for DnDApp {
///     fn update(&mut self, ctx: &Context, frame: &mut Frame) {
///         CentralPanel::default().show(ctx, |ui| {
///             let response = self.dnd.ui(ui, self.items.iter_mut(), |item, ui, handle| {
///                 ui.horizontal(|ui| {
///                     handle.ui(ui, item, |ui| {
///                         ui.label("grab");
///                     });
///                     ui.label(item.clone());
///                 });
///             });
///             if let Some(response) = response.completed {
///                 shift_vec(response.from, response.to, &mut self.items);
///             }
///         });
///     }
/// }
///
/// pub fn main() {
///     use eframe::NativeOptions;
///     let dnd = DragDropUi::default();
///     eframe::run_native("DnD Example", NativeOptions::default(), Box::new(|_| {
///         Box::new(DnDApp {
///             dnd: DragDropUi::default(),
///             items: vec!["a", "b", "c"].into_iter().map(|s| s.to_string()).collect(),
///         })
///     }));
/// }
/// ```
impl DragDropUi {
    pub fn ui<'a, T: DragDropItem + 'a>(
        &mut self,
        ui: &mut Ui,
        items: impl Iterator<Item = &'a T>,
        mut item_ui: impl FnMut(&mut Ui, Handle, usize, &T),
    ) -> DragDropResponse {
        // internal list representation shifted according to previous hover state
        let mut list = items.enumerate().collect::<Vec<_>>();
        if let Some(drag_indices) = self.drag_indices.clone() {
            shift_vec(drag_indices.source, drag_indices.target, &mut list);
        }
        let mut item_rects = Vec::with_capacity(list.len());

        // draw list entries
        let this_list_is_drop_target = self.drag_indices.is_some();
        let list_response = DragDropUi::draw_list(ui, this_list_is_drop_target, |ui| {
            list.iter_mut().for_each(|(idx, item)| {
                // get rect of list entry
                let rect = self.draw_item(ui, item.id(), |ui, handle| {
                    item_ui(ui, handle, *idx, item);
                });
                item_rects.push((*idx, rect));

                // check if this entry is being dragged
                if ui.memory().is_being_dragged(item.id()) {
                    self.set_source_index(*idx);
                }
            });
        });

        // determine target index
        let list_hovered_over = list_response.hovered();
        let hovering_idx = self.determine_hovering_index(ui, list.len(), item_rects);
        if let Some(drag_indices) = &mut self.drag_indices {
            if list_hovered_over && hovering_idx.is_some() {
                // pending [if-let chains](https://github.com/rust-lang/rfcs/blob/master/text/2497-if-let-chains.md#rollout-plan-and-transitioning-to-rust-2018)...
                drag_indices.target = hovering_idx.expect("checked for some in previous line");
            } else {
                // no index being hovered over -> no target
                drag_indices.target = drag_indices.source;
            }
        }

        // return dragging state
        if let Some(drag_indices) = self.drag_indices.clone() {
            // dragging finished
            if ui.input().pointer.any_released() {
                self.drag_indices = None;
                return DragDropResponse::Completed(drag_indices);
            }

            // dragging in progress
            return DragDropResponse::CurrentDrag(drag_indices);
        }
        return DragDropResponse::NoDrag;
    }

    /// Draw the widget for an item using `item_body` either inline with the list or hovering depending
    /// on if its being dragged, then returns its rect. If the item is being dragged, a preview indicator
    /// is drawn in the target list position using the function `drop_place_preview`. If none is provided,
    /// a blank area is reserved in place.
    fn draw_item(
        &mut self,
        ui: &mut Ui,
        id: Id,
        mut item_body: impl FnMut(&mut Ui, Handle),
    ) -> Rect {
        let is_being_dragged = ui.memory().is_being_dragged(id);

        if !is_being_dragged {
            // not dragged -> draw widget to ui
            let scope = ui.scope(|ui| {
                item_body(
                    ui,
                    Handle {
                        state: self,
                        placeholder: false,
                    },
                )
            });
            return scope.response.rect;
        }

        ui.output().cursor_icon = CursorIcon::Grabbing;

        // draw the body to a new layer
        let _layer_id = LayerId::new(Order::Tooltip, id);

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty [`Response`])
        // So this is fine!

        // latest pointer position while dragging
        let pointer_pos = ui
            .ctx()
            .pointer_interact_pos()
            .unwrap_or(ui.next_widget_position());

        // draw hovering item at pointer position
        let hovering_item = egui::Area::new("draggable_item")
            .interactable(false)
            .fixed_pos(pointer_pos + self.drag_delta.unwrap_or(Vec2::default()))
            .show(ui.ctx(), |ui_1| {
                let item_rect = ui_1
                    .scope(|ui_2| {
                        item_body(
                            ui_2,
                            Handle {
                                state: self,
                                placeholder: false,
                            },
                        )
                    })
                    .response
                    .rect;

                return item_rect;
            });

        if true {
            //todo
            let scope = ui.scope(|ui| {
                // disabled style for placeholder ui
                ui.add_enabled_ui(false, |ui| {
                    item_body(
                        ui,
                        Handle {
                            state: self,
                            placeholder: true,
                        },
                    )
                });
            });
            return scope.response.rect;
        } else {
            // allocate space where the item would be
            let (_id, rect) = ui.allocate_space(hovering_item.inner.size());
            return rect;
        }
    }

    /// Draw the list body and todo what other stuff?
    fn draw_list(
        ui: &mut Ui,
        is_drop_target: bool,
        list_body: impl FnOnce(&mut Ui),
    ) -> egui::Response {
        let margin = Vec2::splat(4.0);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);
        let where_to_put_background = ui.painter().add(Shape::Noop);

        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());

        list_body(&mut content_ui);
        let outer_rect =
            Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
        let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

        // determine list coloring depending on wherever this list is currently the drop target
        let style = if is_drop_target && response.hovered() {
            ui.visuals().widgets.active
        } else {
            ui.visuals().widgets.inactive
        };

        ui.painter().set(
            where_to_put_background,
            epaint::RectShape {
                rounding: style.rounding,
                fill: style.bg_fill,
                stroke: style.bg_stroke,
                rect,
            },
        );

        response
    }

    /// Determines the index of the list item that has the closest y position to the current pointer
    /// position. Returns `None` if there is no pointer position (e.g. touch device).
    fn determine_hovering_index(
        &self,
        ui: &Ui,
        list_len: usize,
        item_rects: Vec<(usize, Rect)>,
    ) -> Option<usize> {
        // pointer position
        if let Some(pointer_pos) = ui.input().pointer.hover_pos() {
            let pointer_pos = if let Some(delta) = self.drag_delta {
                pointer_pos + delta
            } else {
                pointer_pos
            };

            // find the closest entry to the pointer position
            // (absolute y distance to top of entry, new entry index, old entry index, entry rect)
            let mut closest: Option<(f32, usize, usize, Rect)> = None;
            let _hovering = item_rects.into_iter().enumerate().for_each(
                |(new_idx, (entry_idx, entry_rect))| {
                    let entry_dist = (entry_rect.top() - pointer_pos.y).abs(); // todo use center().y instead???
                    let val = (entry_dist, new_idx, entry_idx, entry_rect);

                    if let Some((closest_dist, ..)) = closest {
                        if closest_dist > entry_dist {
                            closest = Some(val)
                        }
                    } else {
                        closest = Some(val)
                    }
                },
            );

            if let Some((_dist, new_idx, _original_idx, rect)) = closest {
                // determine hovering index
                let mut hovering_idx = if pointer_pos.y > rect.center().y {
                    new_idx + 1
                } else {
                    new_idx
                };

                if let Some(DragIndices {
                    source: source_idx, ..
                }) = self.drag_indices
                {
                    // account for source being removed
                    if source_idx < hovering_idx && hovering_idx < list_len {
                        hovering_idx += 1;
                    }
                }

                return Some(hovering_idx);
            }
        }
        return None;
    }

    fn set_source_index(&mut self, source_idx: usize) {
        match &mut self.drag_indices {
            Some(drag_indices) => {
                drag_indices.source = source_idx;
            }
            None => {
                self.drag_indices = Some(DragIndices {
                    source: source_idx,
                    target: source_idx,
                })
            }
        };
    }
}
