pub mod handle;
pub mod utils;

use egui::{self, Context, CursorIcon, Id, LayerId, Order, Rect, Sense, Shape, Ui, Vec2};
use epaint::TextureId;
use handle::DragHandle;
use std::hash::Hash;
use utils::shift_slice;

#[derive(Default, Clone, Copy)]
pub struct DragIndices {
    pub source: usize,
    pub target: usize,
}

#[derive(Clone)]
pub enum DragDropResponse {
    NoDrag,
    CurrentDrag(DragIndices),
    Completed(DragIndices),
}

pub trait DragableItem {
    /// Unique id to identify an item in the list.
    fn drag_id(&self) -> Id;
}
impl<T: Hash> DragableItem for T {
    fn drag_id(&self) -> Id {
        Id::new(self)
    }
}

/// [DragDropUi] stores the state of the Drag & Drop list.
///
/// `item_ui` should be a function to draw the ui elements for each item in `items`. Its arguments are:
/// - a mutable reference to the ui
/// - a `DragHandle` that can be used to draw the draggable part of the item ui
/// - the index of the current item in the `items` list
/// - a reference to the current item in the `items` list
///
/// # Example
/// ```rust
/// struct DnDApp {
///     items: Vec<String>,
///     dnd: DragDropUi,
/// }
///
/// impl App for DnDApp {
///     fn update(&mut self, ctx: &Context, frame: &mut Frame) {
///         CentralPanel::default().show(ctx, |ui| {
///             let response = self.dnd.ui(ui, self.items.iter(), |ui, handle, _index, item| {
///                 ui.horizontal(|ui| {
///                     handle.ui(ui, item, |ui| {
///                         ui.label("grab");
///                     });
///                     ui.label(item.clone());
///                 });
///             });
///             if let DragDropResponse::Completed(drag_indices) = response {
///                 shift_vec(drag_indices.source, drag_indices.target, &mut self.items);
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
#[derive(Clone)]
pub struct DragDropUi {
    drag_indices: Option<DragIndices>,
    /// Pointer position relative to the origin of the dragged widget when dragging began
    drag_delta: Option<Vec2>,
    pub draw_drop_preview: bool,
}

impl DragDropUi {
    /// Draws the list of `items` to `ui` using `item_ui` for each item in the list. Returns the
    /// dragging response (to be actioned by the caller).
    pub fn list_ui<'a, T: DragableItem + 'a>(
        &mut self,
        context: &Context,
        ui: &mut Ui,
        items: impl Iterator<Item = &'a T>,
        mut item_ui: impl FnMut(&mut Ui, DragHandle, usize, &T),
    ) -> DragDropResponse {
        // internal list representation shifted according to previous hover state
        let mut list = items.enumerate().collect::<Vec<_>>();

        let list_len = list.len();
        if list_len == 0 {
            return DragDropResponse::NoDrag;
        }

        if let Some(drag_indices) = self.drag_indices {
            let shift_res = shift_slice(drag_indices.source, drag_indices.target, &mut list);

            if let Err(_e) = shift_res {
                // current drag indices are busted!
                let source = drag_indices.source.min(list_len);
                let target = drag_indices.target.min(list_len);
                self.drag_indices = Some(DragIndices { source, target });
            }
        }
        let mut item_rects = Vec::with_capacity(list.len());

        // draw list entries
        let this_list_is_drop_target = self.drag_indices.is_some();
        let list_response = Self::draw_list(ui, this_list_is_drop_target, |ui| {
            list.iter_mut().for_each(|(idx, item)| {
                // get rect of list entry
                let rect = self.draw_item(context, ui, item.drag_id(), |ui, handle| {
                    item_ui(ui, handle, *idx, item);
                });
                item_rects.push((*idx, rect));

                // check if this entry is being dragged
                let is_being_dragged = context.is_being_dragged(item.drag_id());
                if is_being_dragged {
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
            if ui.input(|i| i.pointer.any_released()) {
                self.drag_indices = None;
                return DragDropResponse::Completed(drag_indices);
            }

            // dragging in progress
            return DragDropResponse::CurrentDrag(drag_indices);
        }
        return DragDropResponse::NoDrag;
    }

    /// Draw the list body and _todo: what other stuff?_
    fn draw_list(
        ui: &mut Ui,
        is_drop_target: bool,
        list_body: impl FnOnce(&mut Ui),
    ) -> egui::Response {
        let margin = Vec2::splat(4.0); // todo dpi scaling?

        let outer_rect_bounds = ui.available_rect_before_wrap(); // big ol box
        let inner_rect = outer_rect_bounds.shrink2(margin); // minus margin
        let where_to_put_background = ui.painter().add(Shape::Noop); // assign background shape before drawing list body
        let mut content_ui = ui.child_ui(inner_rect, *ui.layout(), None); // we'll draw list body to child ui thats within margin
                                                                          //let mut content_ui = ui.new_child(ui_builder);

        list_body(&mut content_ui);
        let mut outer_rect = content_ui.min_rect().expand2(margin);
        outer_rect.max.x = content_ui.max_rect().max.x + margin.x; // expand outer box horizontally for padding
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
                rect,
                rounding: style.rounding,
                fill: style.bg_fill,
                stroke: style.bg_stroke,
                blur_width: 0.0,
                // these values disable texture usage
                fill_texture_id: TextureId::default(),
                uv: Rect::ZERO,
            },
        );

        response
    }

    /// Draw the widget for an item using `item_body` either inline with the list or hovering depending
    /// on if its being dragged, then returns its rect. If the item is being dragged, a preview indicator
    /// is drawn in the target list position using the function `drop_place_preview`. If none is provided,
    /// a blank area is reserved in place.
    fn draw_item(
        &mut self,
        context: &Context,
        ui: &mut Ui,
        id: Id,
        mut item_body: impl FnMut(&mut Ui, DragHandle),
    ) -> Rect {
        let is_being_dragged = context.is_being_dragged(id);

        if !is_being_dragged {
            // not dragged -> draw widget to ui
            let scope = ui.scope(|ui| {
                item_body(
                    ui,
                    DragHandle {
                        state: self,
                        placeholder: false,
                    },
                )
            });
            return scope.response.rect;
        }

        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);

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
        let hovering_item = egui::Area::new("draggable_item".into())
            .interactable(false)
            .fixed_pos(pointer_pos + self.drag_delta.unwrap_or(Vec2::default()))
            .show(ui.ctx(), |ui_1| {
                let item_rect = ui_1
                    .scope(|ui_2| {
                        item_body(
                            ui_2,
                            DragHandle {
                                state: self,
                                placeholder: false,
                            },
                        )
                    })
                    .response
                    .rect;

                return item_rect;
            });

        if self.draw_drop_preview {
            let scope = ui.scope(|ui| {
                // disabled style for placeholder ui
                ui.add_enabled_ui(false, |ui| {
                    item_body(
                        ui,
                        DragHandle {
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

    /// Determines the index of the list item that has the closest y position to the current pointer
    /// position. Returns `None` if there is no pointer position (e.g. touch device).
    fn determine_hovering_index(
        &self,
        ui: &Ui,
        list_len: usize,
        item_rects: Vec<(usize, Rect)>,
    ) -> Option<usize> {
        // pointer position
        let hover_pos = ui.input(|i| i.pointer.hover_pos());
        if let Some(pointer_pos) = hover_pos {
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

impl Default for DragDropUi {
    fn default() -> Self {
        Self {
            drag_delta: Default::default(),
            drag_indices: Default::default(),
            draw_drop_preview: true,
        }
    }
}
