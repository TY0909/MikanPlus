//! 验证 taffy 0.9.0 的 flex-grow / stretch 行为(排除 GPUI 集成因素的底层实验)。

use taffy::prelude::*;
use taffy::{FlexDirection, Style, TaffyTree};

#[test]
fn taffy_flex_column_grow() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();

    let head = taffy
        .new_leaf(Style {
            size: Size {
                width: auto(),
                height: length(48.0),
            },
            ..Default::default()
        })
        .unwrap();
    let rest = taffy
        .new_leaf(Style {
            flex_grow: 1.0,
            flex_basis: length(0.0),
            min_size: Size {
                width: auto(),
                height: length(0.0),
            },
            ..Default::default()
        })
        .unwrap();
    let container = taffy
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(1200.0),
                    height: length(800.0),
                },
                ..Default::default()
            },
            &[head, rest],
        )
        .unwrap();

    taffy
        .compute_layout(
            container,
            Size {
                width: length(1200.0),
                height: length(800.0),
            },
        )
        .unwrap();

    let head_layout = taffy.layout(head).unwrap().size;
    let rest_layout = taffy.layout(rest).unwrap().size;
    println!("TAFFY head={:?} rest={:?}", head_layout, rest_layout);
    assert!(
        rest_layout.height > 700.0,
        "taffy flex-grow 应填满剩余高度,实际 {:?}",
        rest_layout
    );
}

#[test]
fn taffy_flex_grow_with_percent_basis() {
    // GPUI 的 flex_1 使用 flex-basis: percent(0),验证 taffy 中是否正常
    let mut taffy: TaffyTree<()> = TaffyTree::new();

    let head = taffy
        .new_leaf(Style {
            size: Size {
                width: auto(),
                height: length(48.0),
            },
            ..Default::default()
        })
        .unwrap();
    let rest = taffy
        .new_leaf(Style {
            flex_grow: 1.0,
            flex_basis: percent(0.0),
            min_size: Size {
                width: auto(),
                height: length(0.0),
            },
            ..Default::default()
        })
        .unwrap();
    let container = taffy
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(1200.0),
                    height: length(800.0),
                },
                ..Default::default()
            },
            &[head, rest],
        )
        .unwrap();

    taffy
        .compute_layout(
            container,
            Size {
                width: length(1200.0),
                height: length(800.0),
            },
        )
        .unwrap();

    let rest_layout = taffy.layout(rest).unwrap().size;
    println!("TAFFY-PCT rest={:?}", rest_layout);
    assert!(
        rest_layout.height > 700.0,
        "taffy percent flex-basis 的 flex-grow 应填满剩余,实际 {:?}",
        rest_layout
    );
}

#[test]
fn taffy_flex_row_stretch() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();

    let only = taffy
        .new_leaf(Style {
            size: Size {
                width: length(300.0),
                height: auto(),
            },
            ..Default::default()
        })
        .unwrap();
    let container = taffy
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: length(1200.0),
                    height: length(800.0),
                },
                ..Default::default()
            },
            &[only],
        )
        .unwrap();

    taffy
        .compute_layout(
            container,
            Size {
                width: length(1200.0),
                height: length(800.0),
            },
        )
        .unwrap();

    let only_layout = taffy.layout(only).unwrap().size;
    println!("TAFFY only={:?}", only_layout);
    assert!(
        only_layout.height > 700.0,
        "taffy 交叉轴 stretch 应填满容器高度,实际 {:?}",
        only_layout
    );
}
