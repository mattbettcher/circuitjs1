use std::collections::VecDeque;
use std::fs;

use circuit_engine::{parse_dump, Circuit, ElementKind};
use eframe::egui::{self, Color32, ComboBox, Key, RichText, Ui};

use crate::draw::{self, Camera, DrawOpts};
use crate::examples::{self, EXAMPLES};
use crate::si;

const SCOPE_CAP: usize = 720;

pub struct CircuitApp {
    circuit: Circuit,
    dump: String,
    example_id: String,
    running: bool,
    steps_per_frame: i32,
    voltage_range: f32,
    show_values: bool,
    show_currents: bool,
    selected: Option<usize>,
    camera: Camera,
    error: Option<String>,
    scope_v: VecDeque<(f64, f64)>,
    scope_i: VecDeque<(f64, f64)>,
    scope_mode: ScopeMode,
    dump_open: bool,
    dump_edit: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeMode {
    Voltage,
    Current,
}

impl CircuitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let example = &EXAMPLES[0];
        let mut app = Self {
            circuit: Circuit::new(),
            dump: example.dump.to_string(),
            example_id: example.id.to_string(),
            running: false,
            steps_per_frame: 40,
            voltage_range: 5.0,
            show_values: true,
            show_currents: true,
            selected: None,
            camera: Camera::default(),
            error: None,
            scope_v: VecDeque::new(),
            scope_i: VecDeque::new(),
            scope_mode: ScopeMode::Voltage,
            dump_open: false,
            dump_edit: example.dump.to_string(),
        };
        app.reload();
        app
    }

    fn load_dump(&mut self, text: &str) {
        self.dump = text.to_string();
        self.dump_edit = text.to_string();
        self.reload();
    }

    fn reload(&mut self) {
        match parse_dump(&self.dump) {
            Ok(c) => {
                self.circuit = c;
                self.error = None;
                self.running = false;
                self.selected = None;
                self.camera.fitted = false;
                self.scope_v.clear();
                self.scope_i.clear();
                if let Err(e) = self.circuit.analyze() {
                    self.error = Some(e.to_string());
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.running = false;
            }
        }
    }

    fn reset_sim(&mut self) {
        self.circuit.reset();
        self.scope_v.clear();
        self.scope_i.clear();
        self.error = None;
        if let Err(e) = self.circuit.analyze() {
            self.error = Some(e.to_string());
        }
    }

    fn step_n(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        match self.circuit.steps(n) {
            Ok(()) => {
                self.error = None;
                self.record_scope();
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.running = false;
            }
        }
    }

    fn record_scope(&mut self) {
        let Some(i) = self.selected else {
            return;
        };
        if i >= self.circuit.element_count() {
            return;
        }
        let volts = self.circuit.element_volts(i);
        let vd = if volts.len() >= 2 {
            volts[0] - volts[1]
        } else {
            volts.first().copied().unwrap_or(0.0)
        };
        let t = self.circuit.t();
        self.scope_v.push_back((t, vd));
        self.scope_i.push_back((t, self.circuit.element_current(i)));
        while self.scope_v.len() > SCOPE_CAP {
            self.scope_v.pop_front();
        }
        while self.scope_i.len() > SCOPE_CAP {
            self.scope_i.pop_front();
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Circuit Engine");
            ui.separator();

            let mut id = self.example_id.clone();
            ComboBox::from_id_salt("example")
                .selected_text(
                    examples::by_id(&id)
                        .map(|e| e.name)
                        .unwrap_or("Custom"),
                )
                .show_ui(ui, |ui| {
                    for ex in EXAMPLES {
                        ui.selectable_value(&mut id, ex.id.to_string(), ex.name);
                    }
                });
            if id != self.example_id {
                if let Some(ex) = examples::by_id(&id) {
                    self.example_id = id;
                    self.load_dump(ex.dump);
                }
            }

            if ui.button("Open dump…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Circuit dump", &["txt"])
                    .pick_file()
                {
                    match fs::read_to_string(&path) {
                        Ok(text) => {
                            self.example_id = "custom".into();
                            self.load_dump(&text);
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
            }
            if ui.button("Edit dump").clicked() {
                self.dump_edit = self.dump.clone();
                self.dump_open = true;
            }

            ui.separator();
            let play = if self.running { "Pause" } else { "Run" };
            if ui.button(play).clicked() {
                self.running = !self.running;
            }
            if ui.button("Step").clicked() {
                self.running = false;
                self.step_n(1);
            }
            if ui.button("Reset").clicked() {
                self.reset_sim();
            }
            if ui.button("Fit").clicked() {
                self.camera.fitted = false;
            }

            ui.separator();
            ui.label("steps/frame");
            ui.add(egui::Slider::new(&mut self.steps_per_frame, 1..=2000).logarithmic(true));
            ui.label("V range");
            ui.add(egui::Slider::new(&mut self.voltage_range, 0.5..=50.0));
            ui.checkbox(&mut self.show_values, "values");
            ui.checkbox(&mut self.show_currents, "currents");
        });

        ui.horizontal(|ui| {
            ui.monospace(format!(
                "t = {:<10}  dt = {:<10}  nodes = {}  islands = {}  elms = {}",
                si::time(self.circuit.t()),
                si::time(self.circuit.time_step()),
                self.circuit.node_count(),
                self.circuit.island_count(),
                self.circuit.element_count(),
            ));
            if let Some(err) = &self.error {
                ui.colored_label(Color32::from_rgb(255, 120, 80), err);
            }
        });
        ui.label(
            RichText::new("Space run/pause · S step · R reset · F fit · scroll zoom · drag pan")
                .small()
                .color(Color32::from_gray(140)),
        );
    }
}

impl eframe::App for CircuitApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.running {
            self.step_n(self.steps_per_frame.max(1) as usize);
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.dump_open {
            if ctx.input(|i| i.key_pressed(Key::Space)) {
                self.running = !self.running;
            }
            if ctx.input(|i| i.key_pressed(Key::S)) {
                self.running = false;
                self.step_n(1);
            }
            if ctx.input(|i| i.key_pressed(Key::R)) {
                self.reset_sim();
            }
            if ctx.input(|i| i.key_pressed(Key::F)) {
                self.camera.fitted = false;
            }
        }

        egui::Panel::top("toolbar").exact_size(72.0).show(ui, |ui| {
            ui.add_space(4.0);
            self.toolbar(ui);
        });

        egui::Panel::bottom("scope")
            .resizable(true)
            .default_size(150.0)
            .min_size(100.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Scope");
                    ui.selectable_value(&mut self.scope_mode, ScopeMode::Voltage, "voltage");
                    ui.selectable_value(&mut self.scope_mode, ScopeMode::Current, "current");
                    if let Some(i) = self.selected {
                        if i < self.circuit.element_count() {
                            let e = &self.circuit.elements[i];
                            ui.label(format!("{}[{i}]", e.kind().name()));
                        }
                    }
                });
                let (samples, label) = match self.scope_mode {
                    ScopeMode::Voltage => (self.scope_v.make_contiguous(), "V"),
                    ScopeMode::Current => (self.scope_i.make_contiguous(), "I"),
                };
                draw::show_scope(ui, samples, label);
            });

        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(260.0)
            .show(ui, |ui| {
                ui.heading("Elements");
                ui.add_space(4.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for i in 0..self.circuit.element_count() {
                        let e = &self.circuit.elements[i];
                        let mut row = e.kind().name().to_string();
                        if let Some((v, u)) = e.primary_value() {
                            row = format!("{row}  {}", si::si(v, u));
                        }
                        if !e.tag().is_empty() {
                            row = format!("{row}  {}", e.tag());
                        }
                        let selected = self.selected == Some(i);
                        if ui.selectable_label(selected, row).clicked() {
                            self.selected = Some(i);
                            self.scope_v.clear();
                            self.scope_i.clear();
                        }
                    }
                });
                ui.separator();
                if let Some(i) = self.selected {
                    if i < self.circuit.element_count() {
                        let e = &self.circuit.elements[i];
                        ui.heading("Selected");
                        ui.label(e.kind().name());
                        if let Some((v, u)) = e.primary_value() {
                            ui.label(si::si(v, u));
                        }
                        let volts = e.volts();
                        if volts.len() >= 2 {
                            ui.monospace(format!("V0 = {}", si::si(volts[0], "V")));
                            ui.monospace(format!("V1 = {}", si::si(volts[1], "V")));
                            ui.monospace(format!("Vd = {}", si::si(volts[0] - volts[1], "V")));
                        } else if let Some(v) = volts.first() {
                            ui.monospace(format!("V = {}", si::si(*v, "V")));
                        }
                        ui.monospace(format!("I  = {}", si::si(e.current(), "A")));
                    }
                } else {
                    ui.weak("Click an element on the schematic.");
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(8, 10, 14)))
            .show(ui, |ui| {
                let opts = DrawOpts {
                    voltage_range: self.voltage_range,
                    show_values: self.show_values,
                    show_currents: self.show_currents,
                    selected: self.selected,
                };
                if let Some(hit) =
                    draw::show_canvas(ui, &self.circuit, &mut self.camera, &opts, self.circuit.t())
                {
                    let kind = self.circuit.elements[hit].kind();
                    if self.selected == Some(hit)
                        && matches!(kind, ElementKind::Switch | ElementKind::SwitchSpdt)
                    {
                        self.circuit.toggle(hit);
                    }
                    if self.selected != Some(hit) {
                        self.selected = Some(hit);
                        self.scope_v.clear();
                        self.scope_i.clear();
                    }
                }
            });

        if self.dump_open {
            let mut open = true;
            egui::Window::new("Circuit dump")
                .open(&mut open)
                .default_size([520.0, 380.0])
                .show(&ctx, |ui| {
                    ui.label("CircuitJS1 text dump (r, c, l, v, i, w, g, d).");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.dump_edit)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(16),
                    );
                    if ui.button("Load").clicked() {
                        self.example_id = "custom".into();
                        self.load_dump(&self.dump_edit.clone());
                        self.dump_open = false;
                    }
                });
            if !open {
                self.dump_open = false;
            }
        }
    }
}
