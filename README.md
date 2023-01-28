# egui_dnd-fork

Fork from [egui_dnd](https://github.com/lucasmerlin/egui_dnd) with tweaks for the use-case of the [Goshenite](https://github.com/lilly-lizard/Goshenite) engine.

_Note: I also found a more expimentatal implimentation of this idea [here](https://github.com/emilk/egui/discussions/1530)._

## TODO

- ability to drag and drop between multiple DragDropUi structs
	- new argument `target_lists: Vec<DragDropUi>`
	- change the way variable `this_list_is_drop_target` is set

# egui_dnd

... is a drag & drop library for [egui](https://github.com/emilk/egui). 

Give it a try here: https://lucasmerlin.github.io/egui_dnd/

To get started, take a look at the [simple example.](https://github.com/lucasmerlin/egui_dnd/blob/main/examples/simple.rs)

![ezgif-2-41c0c5360f](https://user-images.githubusercontent.com/8009393/208403722-b28715cd-b708-4eb4-8d00-36873dee2034.gif)
