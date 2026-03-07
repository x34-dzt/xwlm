use ratatui::widgets::ListItem;

pub fn construct_list_items<'a>(items: &'a [String]) -> Vec<ListItem<'a>> {
    items
        .iter()
        .map(|item| ListItem::new(item.as_str()))
        .collect()
}
