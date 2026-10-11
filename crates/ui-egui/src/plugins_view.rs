//! ArtCraft Store plug-in catalog view: browsing, filtering, and 1-click
//! installing extensions synced directly with `https://akkk09.github.io/artcraft-store/`.

use artcraft_launcher_core::catalog::CraftApp;
use artcraft_launcher_core::plugins::Plugin;
use artcraft_launcher_core::protocol::Command;
use egui::{Color32, CornerRadius, Rect, RichText, Sense, Ui, vec2};

use crate::app::LauncherUi;
use crate::theme::{self, Tokens, rgb};
use crate::widgets::{self, Kind};

const PLUGIN_CARD_MIN_W: f32 = 320.0;
const PLUGIN_CARD_H: f32 = 230.0;
const PLUGIN_CARD_PAD: f32 = 14.0;

impl LauncherUi {
    /// Draws the ArtCraft Store plug-in catalog page.
    pub(crate) fn plugins_view(&mut self, ui: &mut Ui) {
        let t = Tokens::get(ui.ctx());

        // Header / Store sync banner
        egui::Frame::new().fill(t.sunken).stroke(t.hairline()).inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::tag(ui, tl!("ArtCraft Store"), t.accent, Color32::WHITE);
                ui.add_space(8.0);
                ui.label(RichText::new(tl!("Community extensions directly synced with ArtCraft Store")).family(theme::medium()).color(t.ink_strong));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    widgets::hud_link(ui, tl!("Storefront"), artcraft_launcher_core::plugins::STORE_BASE_URL);
                    ui.add_space(12.0);
                    let sync_tip = tl!("Sync plug-ins with ArtCraft Store");
                    if self.syncing_plugins {
                        widgets::nav_icon(ui, "⟳", sync_tip, theme::CONTROL_H, true);
                    } else if widgets::button(ui, tl!("Sync Store"), Kind::Secondary, 0.0).on_hover_text(sync_tip).clicked() {
                        self.syncing_plugins = true;
                        self.backend.send(Command::SyncPlugins);
                    }
                });
            });
        });

        ui.add_space(14.0);

        // Filter and Search Toolbar
        ui.horizontal(|ui| {
            // App selector dropdown
            egui::ComboBox::from_id_salt("plugin_app_filter")
                .selected_text(if self.plugin_app_filter.is_empty() {
                    tl!("All apps").to_string()
                } else {
                    CraftApp::by_id(&self.plugin_app_filter).map_or_else(|| self.plugin_app_filter.clone(), |a| a.name.to_string())
                })
                .width(130.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.plugin_app_filter.is_empty(), tl!("All apps")).clicked() {
                        self.plugin_app_filter.clear();
                    }
                    for app in &artcraft_launcher_core::catalog::SUITE {
                        if ui.selectable_label(self.plugin_app_filter == app.id, app.name).clicked() {
                            self.plugin_app_filter = app.id.to_string();
                        }
                    }
                });

            ui.add_space(8.0);

            // Kind filter dropdown
            egui::ComboBox::from_id_salt("plugin_kind_filter")
                .selected_text(if self.plugin_kind_filter.is_empty() { tl!("All kinds").to_string() } else { self.plugin_kind_filter.clone() })
                .width(110.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.plugin_kind_filter.is_empty(), tl!("All kinds")).clicked() {
                        self.plugin_kind_filter.clear();
                    }
                    for k in ["filter", "plugin", "tool", "template", "preset"] {
                        if ui.selectable_label(self.plugin_kind_filter == k, k).clicked() {
                            self.plugin_kind_filter = k.to_string();
                        }
                    }
                });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.plugin_search)
                        .hint_text(tl!("Search plug-ins"))
                        .desired_width(220.0)
                        .min_size(vec2(0.0, theme::CONTROL_H))
                        .vertical_align(egui::Align::Center)
                        .margin(vec2(10.0, 0.0)),
                );
            });
        });

        ui.add_space(14.0);

        // Catalog Content
        let Some(catalog) = self.plugin_catalog.clone() else {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                if self.syncing_plugins {
                    ui.add(egui::Spinner::new().size(24.0));
                    ui.add_space(12.0);
                    widgets::hud(ui, tl!("Syncing plug-in catalog…"), t.muted);
                } else {
                    widgets::hud(ui, tl!("No plug-in catalog loaded"), t.muted);
                    ui.add_space(12.0);
                    if widgets::button(ui, tl!("Sync Store"), Kind::Primary, 160.0).clicked() {
                        self.syncing_plugins = true;
                        self.backend.send(Command::SyncPlugins);
                    }
                }
            });
            return;
        };

        let search = self.plugin_search.trim().to_lowercase();
        let app_filter = self.plugin_app_filter.trim().to_lowercase();
        let kind_filter = self.plugin_kind_filter.trim().to_lowercase();

        let filtered: Vec<&Plugin> = catalog
            .plugins
            .iter()
            .filter(|p| {
                if !app_filter.is_empty() && !p.app.eq_ignore_ascii_case(&app_filter) {
                    return false;
                }
                if !kind_filter.is_empty() && !p.kind.eq_ignore_ascii_case(&kind_filter) {
                    return false;
                }
                if !search.is_empty() {
                    let in_name = p.name.to_lowercase().contains(&search);
                    let in_desc = p.description.to_lowercase().contains(&search);
                    let in_author = p.author.to_lowercase().contains(&search);
                    let in_tags = p.tags.iter().any(|tag| tag.to_lowercase().contains(&search));
                    if !in_name && !in_desc && !in_author && !in_tags {
                        return false;
                    }
                }
                true
            })
            .collect();

        if filtered.is_empty() {
            ui.add_space(50.0);
            ui.vertical_centered(|ui| {
                widgets::hud(ui, tl!("No plug-ins match"), t.muted);
            });
            return;
        }

        // Plug-in card grid
        let count = filtered.len();
        let avail_w = ui.available_width();
        let cols = ((avail_w + 16.0) / (PLUGIN_CARD_MIN_W + 16.0)).floor().max(1.0) as usize;
        let card_w = ((avail_w - (cols.saturating_sub(1) as f32 * 16.0)) / cols as f32).max(PLUGIN_CARD_MIN_W);

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for row_chunk in filtered.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for plugin in row_chunk {
                        self.draw_plugin_card(ui, plugin, card_w);
                    }
                });
                ui.add_space(16.0);
            }
            ui.add_space(16.0);
            widgets::hud(ui, &format!("{count} plug-ins available"), t.faint);
            ui.add_space(16.0);
        });
    }

    fn draw_plugin_card(&mut self, ui: &mut Ui, plugin: &Plugin, width: f32) {
        let t = Tokens::get(ui.ctx());
        let (rect, resp) = ui.allocate_exact_size(vec2(width, PLUGIN_CARD_H), Sense::hover());
        let painter = ui.painter_at(rect.expand(8.0));
        let hovered = resp.hovered();

        let app_color = CraftApp::by_id(&plugin.app).map_or(Color32::from_rgb(120, 120, 120), |a| rgb(a.color));

        painter.rect(rect, CornerRadius::ZERO, t.bg, t.hairline(), egui::StrokeKind::Inside);
        if hovered {
            painter.rect_filled(Rect::from_min_size(rect.min, vec2(rect.width(), 2.0)), CornerRadius::ZERO, app_color);
        }

        let inner = rect.shrink(PLUGIN_CARD_PAD);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        let ui = &mut child;

        // Top metadata strip: App badge + Kind + Compatibility
        ui.horizontal(|ui| {
            let app_name = CraftApp::by_id(&plugin.app).map_or_else(|| plugin.app.as_str(), |a| a.name);
            widgets::tag(ui, app_name, app_color, Color32::BLACK);
            ui.add_space(6.0);
            widgets::badge(ui, &plugin.kind, t.muted);
            if let Some(compat) = &plugin.compatibility {
                ui.add_space(4.0);
                widgets::badge(ui, compat, t.faint);
            }
        });

        ui.add_space(8.0);

        // Title and author
        ui.label(RichText::new(&plugin.name).font(egui::FontId::new(17.0, theme::display())).color(t.ink_strong));
        let author_line = if plugin.version.is_empty() { plugin.author.clone() } else { format!("v{} · {}", plugin.version, plugin.author) };
        widgets::hud(ui, &author_line, t.muted);

        ui.add_space(6.0);

        // Description
        let desc = if plugin.description.len() > 140 { format!("{}…", &plugin.description[..137]) } else { plugin.description.clone() };
        ui.label(RichText::new(desc).font(egui::FontId::proportional(12.5)).color(t.ink));

        ui.add_space(8.0);

        // Bottom actions row
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                let is_installed = self.installed_plugins.contains_key(&plugin.id);
                let is_busy = self.plugin_installing.contains(&plugin.id);

                if is_busy {
                    ui.add(egui::Spinner::new().size(16.0));
                    widgets::hud(ui, tl!("Installing…"), t.accent);
                } else if is_installed {
                    widgets::badge(ui, tl!("Installed"), t.accent);
                    ui.add_space(4.0);
                    if self.caps.manage_apps {
                        if widgets::button(ui, tl!("Reveal"), Kind::Secondary, 0.0).on_hover_text(tl!("Show in file manager")).clicked() {
                            self.backend.send(Command::RevealPlugin { plugin_id: plugin.id.clone() });
                        }
                        ui.add_space(4.0);
                        if widgets::button(ui, tl!("Uninstall"), Kind::Danger, 0.0).on_hover_text(tl!("Remove plug-in")).clicked() {
                            self.backend.send(Command::UninstallPlugin { plugin_id: plugin.id.clone() });
                        }
                    }
                } else if self.caps.manage_apps {
                    if widgets::button(ui, tl!("Install"), Kind::Primary, 90.0).on_hover_text(tl!("Download and install into app")).clicked() {
                        self.plugin_installing.insert(plugin.id.clone());
                        self.backend.send(Command::InstallPlugin { plugin_id: plugin.id.clone() });
                    }
                } else if let Some(url) = plugin.download_url()
                    && widgets::button(ui, tl!("Download"), Kind::Primary, 90.0).clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    widgets::hud_link(ui, tl!("Source"), &plugin.source_link());
                });
            });
        });
    }
}
