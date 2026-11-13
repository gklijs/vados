//! Notifications and child tiles: the page's own notifications, the side
//! notifications, `latest` tiles, and their reading order.

use super::{escape_html, get_content, MarkdownOptions};
use crate::bulma::{spanning_rows, Color, ImageType, TilePlacement, MAX_ROW_SPAN};
use crate::config_files::{Notification, PageConfig, SpareTile};
use crate::structure::{Item, Structure};
use crate::templates::{
    ContentNotificationTemplate, DisplayDate, ExternalNotificationTemplate,
    InternalNotificationTemplate, SpareTileTemplate,
};
use crate::ui_text::UiText;
use askama::Template;
use std::sync::Arc;

pub(crate) struct ContentItems {
    pub(crate) item: Item,
    pub(crate) left_sub_notifications: Option<Vec<String>>,
    pub(crate) right_sub_notifications: Option<Vec<String>>,
}

pub(crate) fn to_content_items(
    source: &str,
    dir_path: String,
    page_config: PageConfig,
    structure: &Structure,
    options: MarkdownOptions,
) -> ContentItems {
    let path = match dir_path {
        root if root == source => String::from("/"),
        mut d => d.split_off(source.len()),
    };
    let left_sub_notifications = page_config
        .left_notifications
        .as_ref()
        .map(|n| resolve_notifications(source, &path, n, TilePlacement::Left, structure, options));
    let right_sub_notifications = page_config
        .right_notifications
        .as_ref()
        .map(|n| resolve_notifications(source, &path, n, TilePlacement::Right, structure, options));
    let item = Item::new(path, page_config);
    ContentItems {
        item,
        left_sub_notifications,
        right_sub_notifications,
    }
}

fn resolve_notifications(
    source: &str,
    path: &str,
    notifications: &[Notification],
    placement: TilePlacement,
    structure: &Structure,
    options: MarkdownOptions,
) -> Vec<String> {
    notifications
        .iter()
        .enumerate()
        .map(|(i, notification)| {
            resolve_notification(
                source,
                path,
                notification.clone(),
                (placement, i),
                structure,
                options,
            )
        })
        .collect()
}

/// Renders the `index`th notification in the left or right column.
fn resolve_notification(
    source: &str,
    path: &str,
    notification: Notification,
    (placement, index): (TilePlacement, usize),
    structure: &Structure,
    options: MarkdownOptions,
) -> String {
    let id = match placement {
        TilePlacement::Left => format!("sub-l-{}", index),
        TilePlacement::Right => format!("sub-r-{}", index),
        TilePlacement::Side => {
            unreachable!("side notifications are rendered by `items_to_side_notifications`")
        }
    };
    // The first row of child tiles is above the fold.
    let eager = index == 0;
    let content = get_content(source, path, &notification.content, options);
    let image = notification
        .image
        .and_then(|i| structure.process_image(&i, ImageType::Sub { eager }));
    match notification.url {
        None => {
            let color = notification.color.unwrap_or(Color::Info).to_css_class();
            ContentNotificationTemplate {
                title: &notification.title,
                color,
                image,
                content,
                id,
                placement,
            }
            .render()
            .unwrap()
        }
        Some(internal) if internal.starts_with('/') => {
            let color = notification.color.unwrap_or(Color::Link).to_css_class();
            InternalNotificationTemplate {
                title: &notification.title,
                sub_title: &None,
                date: None,
                color,
                url: &internal,
                image,
                content: Some(content),
                id,
                placement,
            }
            .render()
            .unwrap()
        }
        Some(external) => {
            let color = notification.color.unwrap_or(Color::Link).to_css_class();
            ExternalNotificationTemplate {
                title: &notification.title,
                color,
                url: &external,
                image,
                content,
                id,
                placement,
            }
            .render()
            .unwrap()
        }
    }
}

pub(crate) fn items_to_side_notifications(
    items: Vec<Arc<Item>>,
    structure: &Structure,
    ui: &UiText,
) -> Vec<String> {
    let mut result = vec![];
    for (i, item) in items.iter().enumerate() {
        let id = format!("sub-s-{}", i);
        let color = Color::Info.to_css_class();
        let image = item
            .image
            .as_ref()
            .and_then(|i| structure.process_image(i, ImageType::Side));
        let notification = InternalNotificationTemplate {
            title: &Some(item.title.clone()),
            sub_title: &item.sub_title,
            date: item.date.map(|d| DisplayDate::new(&d, ui)),
            color,
            url: &item.path,
            image,
            content: None,
            id,
            placement: TilePlacement::Side,
        }
        .render()
        .unwrap();
        result.push(notification)
    }
    result
}

/// The child tiles row by row, the left one of each pair first: the order
/// the grid places them in and a phone stacks them in. Each tile carries the
/// column it belongs in, so once one column runs out the other's tiles stay
/// put. The rows the shorter column is missing hold the site's spare tile when
/// it has one. Without one its last tile stretches down alongside the others,
/// unless it has an image: then the stretch would be a large empty block, so
/// the cells stay empty. See vados.allium's `ChildTilesPairUp`.
pub(super) fn in_reading_order(
    mut left: Vec<String>,
    mut right: Vec<String>,
    spare_tile: Option<&SpareTile>,
) -> Vec<String> {
    let rows = left.len().max(right.len());
    let (shorter, placement) = if left.len() < right.len() {
        (&mut left, TilePlacement::Left)
    } else {
        (&mut right, TilePlacement::Right)
    };
    let missing = rows - shorter.len();
    if missing > 0 {
        match spare_tile {
            Some(spare_tile) => shorter.push(render_spare_tile(spare_tile, placement, missing)),
            None => {
                if let Some(last) = shorter.last_mut().filter(|t| !t.contains("<img")) {
                    *last = spanning_rows(last, missing + 1);
                }
            }
        }
    }
    let mut tiles = Vec::with_capacity(left.len() + right.len());
    let (mut left, mut right) = (left.into_iter(), right.into_iter());
    loop {
        match (left.next(), right.next()) {
            (None, None) => return tiles,
            (l, r) => tiles.extend(l.into_iter().chain(r)),
        }
    }
}

/// The spare tile, spanning the `rows` cells it fills. Only on a tablet and
/// wider: on a phone tiles stack, so no cell is ever left over.
fn render_spare_tile(spare_tile: &SpareTile, placement: TilePlacement, rows: usize) -> String {
    SpareTileTemplate {
        title: spare_tile.title.as_deref(),
        links: &spare_tile.links,
        start_class: placement.to_column_class(),
        rows: rows.min(MAX_ROW_SPAN),
    }
    .render()
    .unwrap()
}

/// The page's `latest` pages as child tiles, split over the left and right
/// columns the way they pair up: the newest left, the next right, and so on.
/// See vados.allium's `LatestPagesComeFirst`.
pub(super) fn latest_tiles(
    path: &str,
    item: &Item,
    structure: &Structure,
    ui: &UiText,
) -> (Vec<String>, Vec<String>) {
    let Some(latest) = &item.latest else {
        return (vec![], vec![]);
    };
    let pages = structure.get_latest(path, &latest.sources(), latest.depth, latest.get_limit());
    let (mut left, mut right) = (vec![], vec![]);
    for (i, page) in pages.iter().enumerate() {
        let image = page
            .image
            .as_ref()
            .and_then(|r| structure.process_image(r, ImageType::Sub { eager: i < 2 }));
        let content = page
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("<p>{}</p>", escape_html(s)));
        let tile = InternalNotificationTemplate {
            title: &Some(page.title.clone()),
            sub_title: &page.sub_title,
            date: page.date.map(|d| DisplayDate::new(&d, ui)),
            color: Color::Link.to_css_class(),
            url: &page.path,
            image,
            content,
            id: format!("latest-{}", i),
            placement: if i % 2 == 0 {
                TilePlacement::Left
            } else {
                TilePlacement::Right
            },
        }
        .render()
        .unwrap();
        if i % 2 == 0 {
            left.push(tile)
        } else {
            right.push(tile)
        }
    }
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(name: &str, placement: TilePlacement) -> String {
        format!(
            r#"<a class="notification is-link{}" id="{name}">"#,
            placement.to_css_classes()
        )
    }

    #[test]
    fn child_tiles_read_row_by_row_left_first() {
        let left = vec![
            tile("l0", TilePlacement::Left),
            tile("l1", TilePlacement::Left),
        ];
        let right = vec![
            tile("r0", TilePlacement::Right),
            tile("r1", TilePlacement::Right),
        ];
        let ids: Vec<String> = in_reading_order(left, right, None)
            .iter()
            .map(|t| t.rsplit("id=").next().unwrap().to_string())
            .collect();
        assert_eq!(ids, [r#""l0">"#, r#""r0">"#, r#""l1">"#, r#""r1">"#]);
    }

    #[test]
    fn the_shorter_columns_last_tile_spans_the_rows_left_over() {
        let left = (0..4)
            .map(|i| tile(&format!("l{i}"), TilePlacement::Left))
            .collect();
        let right = vec![
            tile("r0", TilePlacement::Right),
            tile("r1", TilePlacement::Right),
        ];
        let tiles = in_reading_order(left, right, None);
        let r1 = tiles.iter().find(|t| t.contains(r#"id="r1""#)).unwrap();
        assert!(
            r1.contains("is-col-span-2-mobile is-row-span-3-tablet\""),
            "{r1}"
        );
        let spanning = tiles.iter().filter(|t| t.contains("is-row-span")).count();
        assert_eq!(spanning, 1);
    }

    #[test]
    fn even_or_one_sided_columns_span_nothing() {
        let even = in_reading_order(
            vec![tile("l0", TilePlacement::Left)],
            vec![tile("r0", TilePlacement::Right)],
            None,
        );
        let one_sided = in_reading_order(
            vec![
                tile("l0", TilePlacement::Left),
                tile("l1", TilePlacement::Left),
            ],
            vec![],
            None,
        );
        assert!(even
            .iter()
            .chain(&one_sided)
            .all(|t| !t.contains("is-row-span")));
    }

    #[test]
    fn a_span_never_exceeds_what_bulma_offers() {
        let left = (0..20)
            .map(|i| tile(&format!("l{i}"), TilePlacement::Left))
            .collect();
        let tiles = in_reading_order(left, vec![tile("r0", TilePlacement::Right)], None);
        assert!(tiles.iter().any(|t| t.contains("is-row-span-12-tablet")));
    }

    fn with_image(name: &str, placement: TilePlacement) -> String {
        format!(r#"{}<img src="/x.webp">"#, tile(name, placement))
    }

    #[test]
    fn a_tile_with_an_image_is_not_stretched_into_an_empty_block() {
        let left = vec![
            tile("l0", TilePlacement::Left),
            tile("l1", TilePlacement::Left),
        ];
        let right = vec![with_image("r0", TilePlacement::Right)];
        let tiles = in_reading_order(left, right, None);
        assert!(tiles.iter().all(|t| !t.contains("is-row-span")));
    }

    fn spare() -> SpareTile {
        serde_json::from_str(
            r#"{"title":"More","links":[{"url":"/archive","title":"Archive & <more>"}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn a_spare_tile_fills_the_missing_cells_and_nothing_stretches() {
        let left = (0..4)
            .map(|i| with_image(&format!("l{i}"), TilePlacement::Left))
            .collect();
        let right = vec![
            tile("r0", TilePlacement::Right),
            tile("r1", TilePlacement::Right),
        ];
        let tiles = in_reading_order(left, right, Some(&spare()));
        assert_eq!(tiles.len(), 7);
        assert!(!tiles[..5].iter().any(|t| t.contains("is-row-span")));
        // Placed right after the shorter column's last tile, on its row.
        let filler = &tiles[5];
        assert!(filler.contains(r#"id="spare-tile""#), "{filler}");
        assert!(filler.contains("is-col-start-2-tablet is-row-span-2-tablet is-hidden-mobile"));
        assert!(
            filler.contains(r#"<a href="/archive">Archive &#38; &#60;more&#62;</a>"#),
            "{filler}"
        );
    }

    #[test]
    fn a_spare_tile_is_left_out_when_the_columns_are_even() {
        let tiles = in_reading_order(
            vec![tile("l0", TilePlacement::Left)],
            vec![tile("r0", TilePlacement::Right)],
            Some(&spare()),
        );
        assert_eq!(tiles.len(), 2);
    }

    #[test]
    fn a_spare_tile_goes_in_the_left_column_when_that_is_shorter() {
        let tiles = in_reading_order(
            vec![],
            vec![tile("r0", TilePlacement::Right)],
            Some(&spare()),
        );
        assert!(tiles[0].contains("is-col-start-1-tablet"), "{}", tiles[0]);
    }

    #[test]
    fn a_right_notification_reads_after_the_latest_tile_on_its_row() {
        // Three latest pages leave the right column one row short: the page's
        // own right notification pairs with the third latest page, so it
        // reads before the left column's notification on a phone. The tile
        // keeps its column; see vados.allium's Notification and
        // ChildTilesPairUp.
        let left = vec![
            tile("latest-0", TilePlacement::Left),
            tile("latest-2", TilePlacement::Left),
            tile("sub-l-0", TilePlacement::Left),
        ];
        let right = vec![
            tile("latest-1", TilePlacement::Right),
            tile("sub-r-0", TilePlacement::Right),
        ];
        let ids: Vec<String> = in_reading_order(left, right, None)
            .iter()
            .map(|t| t.rsplit("id=").next().unwrap().to_string())
            .collect();
        assert_eq!(
            ids,
            [
                r#""latest-0">"#,
                r#""latest-1">"#,
                r#""latest-2">"#,
                r#""sub-r-0">"#,
                r#""sub-l-0">"#
            ]
        );
    }

    #[test]
    fn a_span_is_inserted_before_any_later_classes() {
        let tile = r#"<a class="notification is-link is-col-span-2-mobile extra" id="x">"#;
        let spanned = spanning_rows(tile, 3);
        assert!(
            spanned.contains("is-col-span-2-mobile is-row-span-3-tablet extra"),
            "{spanned}"
        );
    }
}
