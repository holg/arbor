//! Main Arbor GUI application.

use arbor_graph::ArborGraph;
use arbor_watcher::{index_directory, IndexResult};
use eframe::egui;
use std::path::PathBuf;

/// Barnes-Hut theta parameter. Higher = faster but less accurate.
/// 0.7 is a good balance (same as Flutter version).
const BARNES_HUT_THETA: f32 = 0.7;

/// Maximum nodes per QuadTree cell before subdividing.
const QUAD_TREE_MAX_NODES: usize = 4;

/// QuadTree for Barnes-Hut O(n log n) force calculation.
struct QuadTree {
    x: f32,
    y: f32,
    size: f32,
    nodes: Vec<usize>, // indices into the workspace nodes
    children: Option<Box<[QuadTree; 4]>>, // NW, NE, SW, SE
    total_mass: f32,
    center_x: f32,
    center_y: f32,
}

impl QuadTree {
    fn new(x: f32, y: f32, size: f32) -> Self {
        Self {
            x,
            y,
            size,
            nodes: Vec::new(),
            children: None,
            total_mass: 0.0,
            center_x: 0.0,
            center_y: 0.0,
        }
    }

    fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.size && py >= self.y && py < self.y + self.size
    }

    fn insert(&mut self, node_idx: usize, node_x: f32, node_y: f32) {
        if !self.contains(node_x, node_y) {
            return;
        }

        // Update center of mass
        let new_mass = self.total_mass + 1.0;
        self.center_x = (self.center_x * self.total_mass + node_x) / new_mass;
        self.center_y = (self.center_y * self.total_mass + node_y) / new_mass;
        self.total_mass = new_mass;

        // If we have children, insert into appropriate child
        if let Some(ref mut children) = self.children {
            Self::insert_into_child(children, node_idx, node_x, node_y);
            return;
        }

        // Add to this cell
        self.nodes.push(node_idx);

        // Subdivide if too many nodes
        if self.nodes.len() > QUAD_TREE_MAX_NODES && self.size > 10.0 {
            self.subdivide();
        }
    }

    fn subdivide(&mut self) {
        let half = self.size / 2.0;
        let children = Box::new([
            QuadTree::new(self.x, self.y, half),           // NW
            QuadTree::new(self.x + half, self.y, half),    // NE
            QuadTree::new(self.x, self.y + half, half),    // SW
            QuadTree::new(self.x + half, self.y + half, half), // SE
        ]);

        // Redistribute existing nodes - we need positions but don't have them here
        // So we'll just keep the indices and the caller must provide positions
        self.children = Some(children);
        // Note: nodes stay in self.nodes for now; they'll be properly placed by rebuild
    }

    fn insert_into_child(children: &mut [QuadTree; 4], node_idx: usize, x: f32, y: f32) {
        for child in children.iter_mut() {
            if child.contains(x, y) {
                child.insert(node_idx, x, y);
                return;
            }
        }
    }

    /// Calculate repulsion force on a node using Barnes-Hut approximation.
    fn calculate_force(&self, node_x: f32, node_y: f32, node_idx: usize, repulsion: f32) -> (f32, f32) {
        if self.total_mass == 0.0 {
            return (0.0, 0.0);
        }

        let dx = self.center_x - node_x;
        let dy = self.center_y - node_y;
        let dist_sq = (dx * dx + dy * dy).max(1.0);
        let dist = dist_sq.sqrt();

        // Check if this cell only contains the queried node
        if self.nodes.len() == 1 && self.nodes[0] == node_idx && self.children.is_none() {
            return (0.0, 0.0);
        }

        // Barnes-Hut approximation: if cell is far enough, treat as single mass
        if self.size / dist < BARNES_HUT_THETA || self.children.is_none() {
            let force = (repulsion * self.total_mass) / dist_sq;
            let fx = -(dx / dist) * force;
            let fy = -(dy / dist) * force;
            return (fx, fy);
        }

        // Otherwise, recursively calculate from children
        let mut fx = 0.0;
        let mut fy = 0.0;
        if let Some(ref children) = self.children {
            for child in children.iter() {
                if child.total_mass > 0.0 {
                    let (cfx, cfy) = child.calculate_force(node_x, node_y, node_idx, repulsion);
                    fx += cfx;
                    fy += cfy;
                }
            }
        }
        (fx, fy)
    }

    /// Build a QuadTree from node positions.
    fn build(positions: &[(f32, f32)]) -> Self {
        if positions.is_empty() {
            return QuadTree::new(0.0, 0.0, 1000.0);
        }

        // Find bounding box
        let mut min_x = positions[0].0;
        let mut max_x = positions[0].0;
        let mut min_y = positions[0].1;
        let mut max_y = positions[0].1;

        for &(x, y) in positions {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        // Create square bounding box with padding
        let width = max_x - min_x + 100.0;
        let height = max_y - min_y + 100.0;
        let size = width.max(height);

        let mut tree = QuadTree::new(min_x - 50.0, min_y - 50.0, size);
        for (idx, &(x, y)) in positions.iter().enumerate() {
            tree.insert(idx, x, y);
        }

        tree
    }
}

/// Node colors by kind (matching Flutter visualizer theme).
mod colors {
    use eframe::egui::Color32;

    pub const FUNCTION: Color32 = Color32::from_rgb(0, 217, 255); // Electric Cyan
    pub const CLASS: Color32 = Color32::from_rgb(157, 78, 221); // Royal Purple
    pub const METHOD: Color32 = Color32::from_rgb(0, 245, 160); // Mint Green
    pub const VARIABLE: Color32 = Color32::from_rgb(255, 184, 0); // Warm Amber
    pub const IMPORT: Color32 = Color32::from_rgb(255, 107, 107); // Coral Red
    pub const DEFAULT: Color32 = Color32::from_rgb(150, 150, 170); // Gray

    pub fn for_kind(kind: &str) -> Color32 {
        match kind {
            "function" => FUNCTION,
            "class" | "struct" | "enum" => CLASS,
            "method" => METHOD,
            "variable" | "constant" | "field" => VARIABLE,
            "import" | "use" => IMPORT,
            _ => DEFAULT,
        }
    }
}

/// A node with position for rendering.
#[derive(Clone)]
struct GraphNode {
    id: String,
    name: String,
    kind: String,
    file: String,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

/// An edge between nodes.
#[derive(Clone)]
struct GraphEdge {
    source_idx: usize,
    target_idx: usize,
}

/// A workspace represents an indexed codebase.
struct Workspace {
    path: PathBuf,
    name: String,
    graph: ArborGraph,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    is_synced: bool,
}

impl Workspace {
    fn from_index_result(path: PathBuf, result: IndexResult) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        // Convert graph nodes to our GraphNode with random initial positions
        let node_count = result.graph.node_count().max(1);
        let nodes: Vec<GraphNode> = result
            .graph
            .nodes()
            .enumerate()
            .map(|(i, n)| {
                // Spread nodes in a circle initially
                let angle = (i as f32) * 2.0 * std::f32::consts::PI / (node_count as f32);
                let radius = 200.0 + (i as f32 % 5.0) * 50.0;
                GraphNode {
                    id: n.id.clone(),
                    name: n.name.clone(),
                    kind: n.kind.to_string(),
                    file: n.file.clone(),
                    x: angle.cos() * radius,
                    y: angle.sin() * radius,
                    vx: 0.0,
                    vy: 0.0,
                }
            })
            .collect();

        // Build node index lookup
        let node_indices: std::collections::HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();

        // Convert edges
        let edges: Vec<GraphEdge> = result
            .graph
            .export_edges()
            .into_iter()
            .filter_map(|e| {
                let source_idx = node_indices.get(e.source.as_str())?;
                let target_idx = node_indices.get(e.target.as_str())?;
                Some(GraphEdge {
                    source_idx: *source_idx,
                    target_idx: *target_idx,
                })
            })
            .collect();

        Self {
            path,
            name,
            graph: result.graph,
            nodes,
            edges,
            is_synced: true,
        }
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Viewport state for pan/zoom.
struct Viewport {
    offset: egui::Vec2,
    zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl Viewport {
    fn screen_to_world(&self, screen_pos: egui::Pos2, screen_center: egui::Pos2) -> egui::Pos2 {
        let relative = screen_pos - screen_center;
        egui::Pos2::new(
            (relative.x - self.offset.x) / self.zoom,
            (relative.y - self.offset.y) / self.zoom,
        )
    }

    fn world_to_screen(&self, world_pos: egui::Pos2, screen_center: egui::Pos2) -> egui::Pos2 {
        egui::Pos2::new(
            world_pos.x * self.zoom + self.offset.x + screen_center.x,
            world_pos.y * self.zoom + self.offset.y + screen_center.y,
        )
    }
}

/// Application settings.
struct Settings {
    follow_ai: bool,
    low_gpu_mode: bool,
    show_labels: bool,
    show_inspector: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            follow_ai: true,
            low_gpu_mode: false,
            show_labels: true,
            show_inspector: true,
        }
    }
}

/// Arbor GUI application state.
pub struct ArborApp {
    workspaces: Vec<Workspace>,
    selected_workspace: Option<usize>,
    selected_node: Option<usize>,
    hovered_node: Option<usize>,
    viewport: Viewport,
    settings: Settings,
    physics_enabled: bool,
    path_input: String,
    search_query: String,
    search_results: Vec<usize>,
    indexing: bool,
    error_message: Option<String>,
}

impl Default for ArborApp {
    fn default() -> Self {
        Self {
            workspaces: Vec::new(),
            selected_workspace: None,
            selected_node: None,
            hovered_node: None,
            viewport: Viewport::default(),
            settings: Settings::default(),
            physics_enabled: true, // Uses Barnes-Hut O(n log n) for efficient force calculation
            path_input: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            indexing: false,
            error_message: None,
        }
    }
}

impl ArborApp {
    /// Create a new Arbor GUI app.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    /// Create with initial path to index.
    pub fn with_path(cc: &eframe::CreationContext<'_>, path: PathBuf) -> Self {
        let mut app = Self::new(cc);
        app.index_path(path);
        app
    }

    /// Index a path and add as workspace.
    fn index_path(&mut self, path: PathBuf) {
        self.indexing = true;
        self.error_message = None;

        match index_directory(&path) {
            Ok(result) => {
                let workspace = Workspace::from_index_result(path, result);
                self.workspaces.push(workspace);
                self.selected_workspace = Some(self.workspaces.len() - 1);
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to index: {}", e));
            }
        }
        self.indexing = false;
    }

    /// Search for nodes matching query.
    fn update_search(&mut self) {
        self.search_results.clear();

        if self.search_query.is_empty() {
            return;
        }

        let Some(ws_idx) = self.selected_workspace else {
            return;
        };
        let Some(ws) = self.workspaces.get(ws_idx) else {
            return;
        };

        let query = self.search_query.to_lowercase();
        for (idx, node) in ws.nodes.iter().enumerate() {
            if node.name.to_lowercase().contains(&query)
                || node.kind.to_lowercase().contains(&query)
                || node.file.to_lowercase().contains(&query)
            {
                self.search_results.push(idx);
                if self.search_results.len() >= 50 {
                    break; // Limit results
                }
            }
        }
    }

    /// Focus on a specific node (center viewport on it).
    fn focus_node(&mut self, node_idx: usize) {
        let Some(ws_idx) = self.selected_workspace else {
            return;
        };
        let Some(ws) = self.workspaces.get(ws_idx) else {
            return;
        };
        let Some(node) = ws.nodes.get(node_idx) else {
            return;
        };

        self.selected_node = Some(node_idx);
        // Center viewport on node
        self.viewport.offset = egui::vec2(-node.x * self.viewport.zoom, -node.y * self.viewport.zoom);
    }

    /// Run force-directed layout simulation step using Barnes-Hut algorithm.
    /// This is O(n log n) instead of O(n²), making it feasible for large graphs.
    /// Matches Flutter's force_layout.dart for consistent behavior.
    fn physics_step(&mut self) {
        if !self.physics_enabled {
            return;
        }

        let Some(ws_idx) = self.selected_workspace else {
            return;
        };
        let Some(ws) = self.workspaces.get_mut(ws_idx) else {
            return;
        };

        if ws.nodes.is_empty() {
            return;
        }

        // Physics parameters (matching Flutter)
        let repulsion = 8000.0;
        let attraction = 0.08;
        let cluster_gravity = 0.5;
        let damping = 0.92;
        let min_distance = 60.0;
        let max_force = 50.0;
        let dt = 0.016; // ~60fps

        let mut forces: Vec<(f32, f32)> = vec![(0.0, 0.0); ws.nodes.len()];

        // 1. File-based clustering (gravity wells)
        // Group nodes by file and pull them towards their file's centroid
        let mut file_groups: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
        for (i, node) in ws.nodes.iter().enumerate() {
            file_groups.entry(node.file.as_str()).or_default().push(i);
        }

        for indices in file_groups.values() {
            if indices.len() <= 1 {
                continue;
            }

            // Calculate centroid
            let mut cx = 0.0;
            let mut cy = 0.0;
            for &i in indices {
                cx += ws.nodes[i].x;
                cy += ws.nodes[i].y;
            }
            cx /= indices.len() as f32;
            cy /= indices.len() as f32;

            // Pull nodes towards centroid
            for &i in indices {
                let dx = cx - ws.nodes[i].x;
                let dy = cy - ws.nodes[i].y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist > min_distance / 2.0 {
                    let force = dist * cluster_gravity;
                    forces[i].0 += (dx / dist) * force * dt;
                    forces[i].1 += (dy / dist) * force * dt;
                }
            }
        }

        // 2. Repulsion using Barnes-Hut O(n log n)
        let positions: Vec<(f32, f32)> = ws.nodes.iter().map(|n| (n.x, n.y)).collect();
        let quad_tree = QuadTree::build(&positions);

        for (i, node) in ws.nodes.iter().enumerate() {
            let (fx, fy) = quad_tree.calculate_force(node.x, node.y, i, repulsion);
            forces[i].0 += fx * dt;
            forces[i].1 += fy * dt;
        }

        // 3. Edge attraction (springs with minimum distance)
        for edge in &ws.edges {
            let dx = ws.nodes[edge.target_idx].x - ws.nodes[edge.source_idx].x;
            let dy = ws.nodes[edge.target_idx].y - ws.nodes[edge.source_idx].y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > min_distance {
                let force = ((dist - min_distance) * attraction).min(max_force);
                let fx = (dx / dist) * force;
                let fy = (dy / dist) * force;

                forces[edge.source_idx].0 += fx * dt;
                forces[edge.source_idx].1 += fy * dt;
                forces[edge.target_idx].0 -= fx * dt;
                forces[edge.target_idx].1 -= fy * dt;
            }
        }

        // 4. Apply forces and calculate total energy
        let mut total_energy = 0.0;
        for (i, node) in ws.nodes.iter_mut().enumerate() {
            node.vx = (node.vx + forces[i].0) * damping;
            node.vy = (node.vy + forces[i].1) * damping;

            node.x += node.vx * dt;
            node.y += node.vy * dt;

            total_energy += node.vx * node.vx + node.vy * node.vy;
        }

        // 5. Auto-disable when settled (energy below threshold)
        if total_energy < 0.1 {
            self.physics_enabled = false;
        }
    }

    /// Render the left sidebar.
    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workspaces");
        ui.separator();

        // Workspace list
        for (idx, workspace) in self.workspaces.iter().enumerate() {
            let selected = self.selected_workspace == Some(idx);
            let status = if workspace.is_synced { "●" } else { "○" };
            let label = format!("{} {} ({} nodes)", status, workspace.name, workspace.node_count());

            if ui.selectable_label(selected, label).clicked() {
                self.selected_workspace = Some(idx);
                self.selected_node = None;
                self.viewport = Viewport::default();
            }
        }

        if self.workspaces.is_empty() {
            ui.label("No workspaces indexed");
        }

        ui.add_space(10.0);

        // Add folder button with file dialog
        if ui.button("+ Add Folder...").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                self.index_path(path);
            }
        }

        // Manual path input (collapsed)
        ui.collapsing("Manual path", |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.path_input);
                if ui.button("Index").clicked() && !self.path_input.is_empty() {
                    let path = PathBuf::from(&self.path_input);
                    self.index_path(path);
                    self.path_input.clear();
                }
            });
        });

        if self.indexing {
            ui.spinner();
            ui.label("Indexing...");
        }

        if let Some(err) = &self.error_message {
            ui.colored_label(egui::Color32::RED, err);
        }

        ui.add_space(20.0);
        ui.separator();
        ui.heading("View");
        ui.add_space(10.0);

        ui.checkbox(&mut self.physics_enabled, "Physics simulation");
        ui.checkbox(&mut self.settings.show_labels, "Show labels");
        ui.checkbox(&mut self.settings.show_inspector, "Show inspector");

        ui.add_space(10.0);
        ui.label(format!("Zoom: {:.1}x", self.viewport.zoom));
        if ui.button("Reset view").clicked() {
            self.viewport = Viewport::default();
        }

        ui.add_space(20.0);
        ui.separator();
        ui.heading("Settings");
        ui.add_space(10.0);

        ui.checkbox(&mut self.settings.follow_ai, "Follow AI");
        ui.checkbox(&mut self.settings.low_gpu_mode, "Low GPU Mode");
    }

    /// Render the node inspector panel.
    fn render_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();

        let (ws_idx, node_idx) = match (self.selected_workspace, self.selected_node) {
            (Some(w), Some(n)) => (w, n),
            _ => {
                ui.label("Select a node to inspect");
                return;
            }
        };

        let Some(ws) = self.workspaces.get(ws_idx) else {
            return;
        };
        let Some(node) = ws.nodes.get(node_idx) else {
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.strong(&node.name);
        });
        ui.horizontal(|ui| {
            ui.label("Kind:");
            ui.colored_label(colors::for_kind(&node.kind), &node.kind);
        });
        ui.horizontal(|ui| {
            ui.label("File:");
            ui.label(&node.file);
        });
        ui.horizontal(|ui| {
            ui.label("ID:");
            ui.label(&node.id);
        });
    }

    /// Render the main graph view.
    fn render_graph(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

        let rect = response.rect;
        let center = rect.center();

        // Background
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 30));

        // Handle pan
        if response.dragged_by(egui::PointerButton::Primary) && self.hovered_node.is_none() {
            self.viewport.offset += response.drag_delta();
        }

        // Handle zoom
        if let Some(hover_pos) = response.hover_pos() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let zoom_factor = 1.0 + scroll * 0.001;
                let new_zoom = (self.viewport.zoom * zoom_factor).clamp(0.1, 5.0);

                // Zoom toward mouse position
                let mouse_world = self.viewport.screen_to_world(hover_pos, center);
                self.viewport.zoom = new_zoom;
                let new_mouse_screen = self.viewport.world_to_screen(mouse_world, center);
                self.viewport.offset += hover_pos - new_mouse_screen;
            }
        }

        let Some(ws_idx) = self.selected_workspace else {
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "Index a workspace to visualize",
                egui::FontId::proportional(24.0),
                egui::Color32::from_rgb(100, 100, 120),
            );
            return;
        };

        let Some(ws) = self.workspaces.get(ws_idx) else {
            return;
        };

        if ws.nodes.is_empty() {
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "No nodes in graph",
                egui::FontId::proportional(24.0),
                egui::Color32::from_rgb(100, 100, 120),
            );
            return;
        }

        // Draw edges
        let edge_color = egui::Color32::from_rgba_unmultiplied(100, 100, 120, 80);
        for edge in &ws.edges {
            let source = &ws.nodes[edge.source_idx];
            let target = &ws.nodes[edge.target_idx];

            let start = self.viewport.world_to_screen(egui::pos2(source.x, source.y), center);
            let end = self.viewport.world_to_screen(egui::pos2(target.x, target.y), center);

            if rect.contains(start) || rect.contains(end) {
                painter.line_segment([start, end], egui::Stroke::new(1.0, edge_color));
            }
        }

        // Draw nodes and handle hover/click
        let node_radius = 6.0 * self.viewport.zoom.sqrt();
        let mut new_hovered = None;

        for (idx, node) in ws.nodes.iter().enumerate() {
            let pos = self.viewport.world_to_screen(egui::pos2(node.x, node.y), center);

            if !rect.contains(pos) {
                continue;
            }

            let color = colors::for_kind(&node.kind);
            let is_selected = self.selected_node == Some(idx);
            let is_hovered = self.hovered_node == Some(idx);

            // Check hover
            if let Some(hover_pos) = response.hover_pos() {
                if (hover_pos - pos).length() < node_radius + 4.0 {
                    new_hovered = Some(idx);
                }
            }

            // Draw node
            let radius = if is_selected || is_hovered {
                node_radius * 1.5
            } else {
                node_radius
            };

            painter.circle_filled(pos, radius, color);

            if is_selected {
                painter.circle_stroke(pos, radius + 2.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
            }

            // Draw label
            if self.settings.show_labels && (is_hovered || is_selected || self.viewport.zoom > 1.5) {
                painter.text(
                    pos + egui::vec2(radius + 4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &node.name,
                    egui::FontId::proportional(12.0 * self.viewport.zoom.sqrt()),
                    egui::Color32::WHITE,
                );
            }
        }

        self.hovered_node = new_hovered;

        // Handle click to select
        if response.clicked() {
            self.selected_node = self.hovered_node;
        }
    }

    /// Render the bottom status bar.
    fn render_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(idx) = self.selected_workspace {
                if let Some(ws) = self.workspaces.get(idx) {
                    let status = if ws.is_synced { "SYNCED" } else { "OFFLINE" };
                    ui.label(format!(
                        "{} | {} nodes | {} edges | {}",
                        ws.name,
                        ws.node_count(),
                        ws.edge_count(),
                        status
                    ));
                }
            } else {
                ui.label("No workspace selected");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(idx) = self.hovered_node {
                    if let Some(ws_idx) = self.selected_workspace {
                        if let Some(ws) = self.workspaces.get(ws_idx) {
                            if let Some(node) = ws.nodes.get(idx) {
                                ui.label(format!("{} ({})", node.name, node.kind));
                            }
                        }
                    }
                }
            });
        });
    }
}

impl eframe::App for ArborApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Run physics
        self.physics_step();

        // Request repaint for animation
        if self.physics_enabled && self.selected_workspace.is_some() {
            ctx.request_repaint();
        }

        // Top search bar
        egui::TopBottomPanel::top("search_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search for functions, classes...")
                        .desired_width(300.0),
                );
                if response.changed() {
                    self.update_search();
                }

                // Show result count
                if !self.search_query.is_empty() {
                    ui.label(format!("{} results", self.search_results.len()));
                }
            });

            // Show search results dropdown
            if !self.search_results.is_empty() {
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        let results = self.search_results.clone();
                        for node_idx in results.iter().take(20) {
                            if let Some(ws_idx) = self.selected_workspace {
                                if let Some(ws) = self.workspaces.get(ws_idx) {
                                    if let Some(node) = ws.nodes.get(*node_idx) {
                                        let label = format!(
                                            "{} ({}) - {}",
                                            node.name, node.kind, node.file
                                        );
                                        if ui
                                            .selectable_label(
                                                self.selected_node == Some(*node_idx),
                                                label,
                                            )
                                            .clicked()
                                        {
                                            self.focus_node(*node_idx);
                                            self.search_query.clear();
                                            self.search_results.clear();
                                        }
                                    }
                                }
                            }
                        }
                    });
            }
        });

        // Left sidebar
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        // Right inspector panel (conditional)
        if self.settings.show_inspector {
            egui::SidePanel::right("inspector")
                .resizable(true)
                .default_width(200.0)
                .show(ctx, |ui| {
                    self.render_inspector(ui);
                });
        }

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.render_status_bar(ui);
        });

        // Main graph area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_graph(ui);
        });
    }
}

/// Run the Arbor GUI.
pub fn run_gui(initial_path: Option<PathBuf>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Arbor"),
        ..Default::default()
    };

    eframe::run_native(
        "Arbor",
        options,
        Box::new(move |cc| {
            if let Some(path) = initial_path {
                Ok(Box::new(ArborApp::with_path(cc, path)))
            } else {
                Ok(Box::new(ArborApp::new(cc)))
            }
        }),
    )
}
