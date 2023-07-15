use super::*;

pub fn menu_item(pipeline: PipelineRef<Option<u8>>, item_idx: u8) -> PipelineRef<bool> {

  let fun = Box::new(move |selected_item, _, _, _: &mut Vec<Action>| {
    if let Some(selected_item) = selected_item {
      selected_item == item_idx
    } else {
      false
    }
  });

  let p = FnStage::from("menu_item", format!("item: {}", item_idx), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
