use egui::{Frame, Label, Margin, RichText, Ui, Widget};
use parsers::details::*;
use stypes::GrabbedElement;

use crate::session::ui::shared::SessionShared;

#[derive(Debug, Default)]
pub struct DetailsUI {
    pub message: Option<StructuredLogMessage>,
    pub selected: Option<u64>,
}

impl DetailsUI {
    pub fn handle_selected_log(&mut self, selected_row: Option<u64>, log: GrabbedElement) {
        if selected_row.is_some_and(|row| row == log.pos as u64) {
            // TODO
            println!("=> selected row: {}", log.pos);
            let bytes = log.content.into_bytes();
            let message = StructuredLogMessage {
                root: DetailNode {
                    id: 0,
                    name: String::from("PAYLOAD"),
                    role: NodeRole::Payload,
                    value: None,
                    byte_range: Some(ByteRange {
                        offset: 0,
                        length: bytes.len(),
                    }),
                    bit_range: None,
                    children: vec![],
                },
                bytes: Some(bytes),
            };
            self.selected = Some(message.root.id);
            self.message = Some(message);
        }
    }

    pub fn render_content(&mut self, shared: &SessionShared, ui: &mut Ui) {
        if shared.logs.selected_count() != 1 {
            return;
        }

        // TODO
        if let Some(message) = &self.message {
            egui::Panel::left("protocol_tree")
                .resizable(true)
                .default_size(400.0)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            show_detail_node(
                                ui,
                                &message.root,
                                &mut self.selected,
                                message.bytes.as_deref(),
                            );
                        });
                });

            let selected_range = find_selected_range(&message.root, self.selected);

            egui::CentralPanel::default().show_inside(ui, |ui| {
                show_bytes(ui, message.bytes.as_deref(), selected_range);
            });
        }
    }
}

fn show_detail_node(
    ui: &mut egui::Ui,
    node: &DetailNode,
    selected: &mut Option<u64>,
    bytes: Option<&[u8]>,
) {
    let is_selected = *selected == Some(node.id);
    let label = node_label(node);

    if node.children.is_empty() {
        ui.horizontal(|ui| {
            let response = ui.selectable_label(is_selected, label);

            if response.clicked() {
                *selected = Some(node.id);
            }

            response.context_menu(|ui| {
                show_range_context_menu(ui, node, bytes);
            });

            show_range(ui, node);
        });

        return;
    }

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        egui::Id::new(("detail_node", node.id)),
        false,
    )
    .show_header(ui, |ui| {
        let response = ui.selectable_label(is_selected, label);

        if response.clicked() {
            *selected = Some(node.id);
        }

        response.context_menu(|ui| {
            show_range_context_menu(ui, node, bytes);
        });

        show_range(ui, node);
    })
    .body(|ui| {
        for child in &node.children {
            show_detail_node(ui, child, selected, bytes);
        }
    });
}

fn node_id(node: &DetailNode) -> egui::Id {
    egui::Id::new(node as *const DetailNode)
}

fn find_selected_range(node: &DetailNode, selected: Option<u64>) -> Option<ByteRange> {
    let selected = selected?;

    if node.id == selected {
        return node.byte_range;
    }

    for child in &node.children {
        if let Some(range) = find_selected_range(child, Some(selected)) {
            return Some(range);
        }
    }

    None
}

fn node_label(node: &DetailNode) -> String {
    match &node.value {
        Some(value) => format!("{}: {}", node.name, value_label(value)),
        None => node.name.clone(),
    }
}

fn value_label(value: &Value) -> String {
    match value {
        Value::Bool(value) => value.to_string(),
        Value::UInt(value) => value.to_string(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::String(value) => value.clone(),

        Value::Bytes(value) => value
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" "),

        Value::Enum { raw, name } => {
            format!("{name} ({raw})")
        }

        Value::BitMask { raw, flags } => {
            let set_flags = flags
                .iter()
                .filter(|flag| flag.set)
                .map(|flag| flag.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            format!("{raw} [{set_flags}]")
        }
    }
}

fn show_range(ui: &mut egui::Ui, node: &DetailNode) {
    if let Some(range) = node.byte_range {
        ui.weak(format!(
            "[{}..{}]",
            range.offset,
            range.offset + range.length
        ));
    }

    if let Some(range) = node.bit_range {
        ui.weak(format!(
            "|{}..{}|",
            range.offset,
            range.offset + range.length
        ));
    }
}

const BYTES_PER_ROW: usize = 16;
const MAX_DISPLAY_BYTES: usize = 512;

fn show_bytes(ui: &mut egui::Ui, bytes: Option<&[u8]>, selected_range: Option<ByteRange>) {
    let Some(bytes) = bytes else {
        ui.weak("No raw bytes available");
        return;
    };

    let Some(range) = selected_range else {
        ui.weak("No node selected");
        return;
    };

    let start = range.offset.min(bytes.len());
    let end = range.offset.saturating_add(range.length).min(bytes.len());

    let display_end = start.saturating_add(MAX_DISPLAY_BYTES).min(end);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            show_hex_view(ui, bytes, start, display_end);

            if display_end < end {
                ui.add_space(4.0);
                ui.weak(format!("... {} more bytes", end - display_end));
            }
        });
}

fn show_hex_view(ui: &mut egui::Ui, bytes: &[u8], start: usize, end: usize) {
    let first_row = (start / BYTES_PER_ROW) * BYTES_PER_ROW;

    let weak = ui.visuals().weak_text_color();
    let normal = ui.visuals().text_color();

    // header
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{:>8}", ""))
                .monospace()
                .color(weak),
        );

        ui.add_space(8.0);

        for column in 0..BYTES_PER_ROW {
            if column == 8 {
                ui.add_space(8.0);
            }

            ui.label(
                egui::RichText::new(format!("{column:02X}"))
                    .monospace()
                    .color(weak),
            );
        }
    });

    // rows
    for row_start in (first_row..end).step_by(BYTES_PER_ROW) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{row_start:08X}"))
                    .monospace()
                    .color(weak),
            );

            ui.add_space(8.0);

            for column in 0..BYTES_PER_ROW {
                if column == 8 {
                    ui.add_space(8.0);
                }

                let offset = row_start + column;

                if offset < start || offset >= end {
                    ui.label(egui::RichText::new("  ").monospace().color(weak));
                } else {
                    ui.label(
                        egui::RichText::new(format!("{:02X}", bytes[offset]))
                            .monospace()
                            .color(normal),
                    );
                }
            }
        });
    }
}

fn show_range_context_menu(ui: &mut egui::Ui, node: &DetailNode, bytes: Option<&[u8]>) {
    let Some(bytes) = bytes else {
        return;
    };

    let Some(range) = node.byte_range else {
        return;
    };

    let start = range.offset.min(bytes.len());
    let end = range.offset.saturating_add(range.length).min(bytes.len());

    let range_bytes = &bytes[start..end];

    if ui.button("Copy as HEX").clicked() {
        let text = range_bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        ui.ctx().copy_text(text);
        ui.close();
    }

    if ui.button("Export as Bytes").clicked() {
        let path = format!("range_{:06X}_{:06X}.bin", start, end,);

        // TODO file dialog
        if let Err(err) = std::fs::write(&path, range_bytes) {
            eprintln!("Failed to write {path}: {err}");
        } else {
            println!("Exported raw bytes to {path}");
        }

        ui.close();
    }
}
