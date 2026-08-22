use circuit_engine::{Circuit, ElementKind};
use eframe::egui::{
    self, ecolor::Hsva, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2,
};

use crate::si;

pub struct Camera {
    pub origin: Pos2,
    pub zoom: f32,
    pub fitted: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            origin: Pos2::new(50.0, 50.0),
            zoom: 3.0,
            fitted: false,
        }
    }
}

impl Camera {
    pub fn to_screen(&self, p: (i32, i32), rect: Rect) -> Pos2 {
        let c = Pos2::new(p.0 as f32, p.1 as f32);
        rect.center() + (c - self.origin) * self.zoom
    }

    pub fn to_circuit(&self, screen: Pos2, rect: Rect) -> Pos2 {
        self.origin + (screen - rect.center()) / self.zoom
    }

    pub fn fit(&mut self, circuit: &Circuit, rect: Rect) {
        let mut min = Pos2::new(f32::MAX, f32::MAX);
        let mut max = Pos2::new(f32::MIN, f32::MIN);
        let mut any = false;
        for elm in &circuit.elements {
            for &(x, y) in elm.geometry() {
                any = true;
                min.x = min.x.min(x as f32);
                min.y = min.y.min(y as f32);
                max.x = max.x.max(x as f32);
                max.y = max.y.max(y as f32);
            }
        }
        if !any || rect.width() < 8.0 || rect.height() < 8.0 {
            return;
        }
        let pad = 24.0;
        let bw = (max.x - min.x).max(16.0);
        let bh = (max.y - min.y).max(16.0);
        self.zoom = ((rect.width() - pad * 2.0) / bw)
            .min((rect.height() - pad * 2.0) / bh)
            .clamp(0.8, 12.0);
        self.origin = Pos2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5);
        self.fitted = true;
    }
}

pub struct DrawOpts {
    pub voltage_range: f32,
    pub show_values: bool,
    pub show_currents: bool,
    pub selected: Option<usize>,
}

/// CircuitJS1-style voltage rainbow: blue (neg) → green → red (pos).
pub fn voltage_color(v: f64, range: f32) -> Color32 {
    if !v.is_finite() {
        return Color32::from_gray(80);
    }
    let r = range.max(0.1) as f64;
    let t = ((v / r + 1.0) * 0.5).clamp(0.0, 1.0) as f32;
    let hue = (1.0 - t) * (240.0 / 360.0);
    Hsva::new(hue, 0.82, 1.0, 1.0).into()
}

pub fn show_canvas(
    ui: &mut Ui,
    circuit: &Circuit,
    cam: &mut Camera,
    opts: &DrawOpts,
    t: f64,
) -> Option<usize> {
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = resp.rect;
    painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(8, 10, 14));

    if !cam.fitted {
        cam.fit(circuit, rect);
    }

    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.1 {
            cam.zoom = (cam.zoom * (1.0 + scroll * 0.002)).clamp(0.4, 24.0);
            if let Some(hover) = resp.hover_pos() {
                let before = cam.to_circuit(hover, rect);
                cam.origin += before - cam.to_circuit(hover, rect);
            }
        }
    }

    let mut hit: Option<(usize, f32)> = None;
    if let Some(pos) = resp.hover_pos() {
        for (i, elm) in circuit.elements.iter().enumerate() {
            let g = elm.geometry();
            if g.is_empty() {
                continue;
            }
            let screens: Vec<Pos2> = g.iter().map(|&p| cam.to_screen(p, rect)).collect();
            for a in 0..screens.len() {
                for b in (a + 1)..screens.len() {
                    let d = dist_to_segment(pos, screens[a], screens[b]);
                    if d < 10.0 && hit.map(|(_, best)| d < best).unwrap_or(true) {
                        hit = Some((i, d));
                    }
                }
            }
        }
    }

    let mut clicked = None;
    if resp.clicked() {
        clicked = hit.map(|(i, _)| i);
    }

    if resp.dragged() {
        cam.origin -= resp.drag_delta() / cam.zoom;
    }

    draw_grid(&painter, cam, rect);

    for (i, elm) in circuit.elements.iter().enumerate() {
        let g = elm.geometry();
        if g.len() < 2 {
            continue;
        }
        let pts: Vec<Pos2> = g.iter().map(|&p| cam.to_screen(p, rect)).collect();
        let selected = opts.selected == Some(i);
        draw_element(
            &painter,
            elm.kind(),
            &pts,
            elm.volts(),
            elm.current(),
            elm.primary_value(),
            elm.tag(),
            opts,
            selected,
            t,
            cam.zoom,
        );
    }

    clicked
}

fn draw_grid(painter: &egui::Painter, cam: &Camera, rect: Rect) {
    let step = 16.0;
    let c0 = cam.to_circuit(rect.left_top(), rect);
    let c1 = cam.to_circuit(rect.right_bottom(), rect);
    let color = Color32::from_rgb(22, 26, 34);
    let x0 = (c0.x / step).floor() * step;
    let y0 = (c0.y / step).floor() * step;
    let mut x = x0;
    while x <= c1.x + step {
        let a = cam.to_screen((x as i32, c0.y as i32), rect);
        let b = cam.to_screen((x as i32, c1.y as i32), rect);
        painter.line_segment(
            [Pos2::new(a.x, rect.top()), Pos2::new(b.x, rect.bottom())],
            Stroke::new(1.0, color),
        );
        x += step;
    }
    let mut y = y0;
    while y <= c1.y + step {
        let a = cam.to_screen((c0.x as i32, y as i32), rect);
        let b = cam.to_screen((c1.x as i32, y as i32), rect);
        painter.line_segment(
            [Pos2::new(rect.left(), a.y), Pos2::new(rect.right(), b.y)],
            Stroke::new(1.0, color),
        );
        y += step;
    }
}

fn draw_element(
    painter: &egui::Painter,
    kind: ElementKind,
    pts: &[Pos2],
    volts: &[f64],
    current: f64,
    value: Option<(f64, &'static str)>,
    tag: &str,
    opts: &DrawOpts,
    selected: bool,
    t: f64,
    zoom: f32,
) {
    let p1 = pts[0];
    let p2 = pts[1];
    let v0 = volts.first().copied().unwrap_or(0.0);
    let v1 = volts.get(1).copied().unwrap_or(v0);
    let w = (1.6 * (zoom / 3.0)).clamp(1.4, 3.2);
    let hl = if selected {
        Some(Stroke::new(w + 3.0, Color32::from_rgb(255, 210, 70)))
    } else {
        None
    };
    if let Some(s) = hl {
        painter.line_segment([p1, p2], s);
        for extra in pts.iter().skip(2) {
            painter.line_segment([p1, *extra], s);
        }
    }

    match kind {
        ElementKind::Wire => {
            stroke_grad(painter, p1, p2, v0, v1, opts.voltage_range, w);
        }
        ElementKind::Resistor => {
            let (l1, l2) = leads(p1, p2, 32.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            zigzag(
                painter,
                l1,
                l2,
                v0,
                v1,
                opts.voltage_range,
                w,
                6.0 * zoom / 3.0,
            );
        }
        ElementKind::Capacitor => {
            let (l1, l2) = leads(p1, p2, 10.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            plate(
                painter,
                l1,
                p2 - p1,
                v0,
                opts.voltage_range,
                w,
                12.0 * zoom / 3.0,
            );
            plate(
                painter,
                l2,
                p2 - p1,
                v1,
                opts.voltage_range,
                w,
                12.0 * zoom / 3.0,
            );
        }
        ElementKind::Inductor => {
            let (l1, l2) = leads(p1, p2, 32.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            coils(painter, l1, l2, v0, v1, opts.voltage_range, w);
        }
        ElementKind::Voltage => {
            let (l1, l2) = leads(p1, p2, 28.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            let c = l1.lerp(l2, 0.5);
            let r = l1.distance(l2) * 0.5;
            painter.circle_stroke(
                c,
                r,
                Stroke::new(w, voltage_color((v0 + v1) * 0.5, opts.voltage_range)),
            );
            let dir = (p2 - p1).normalized();
            let plus = c + dir * r * 0.35;
            let minus = c - dir * r * 0.35;
            let tick = dir.rot90() * (r * 0.22);
            painter.line_segment([plus - tick, plus + tick], Stroke::new(w, Color32::WHITE));
            painter.line_segment(
                [plus - dir * r * 0.22, plus + dir * r * 0.22],
                Stroke::new(w, Color32::WHITE),
            );
            painter.line_segment([minus - tick, minus + tick], Stroke::new(w, Color32::WHITE));
        }
        ElementKind::Rail => {
            stroke_grad(painter, p1, p2, v0, v0, opts.voltage_range, w);
            painter.circle_filled(p2, 7.0 * zoom / 3.0, voltage_color(v0, opts.voltage_range));
            painter.circle_stroke(p2, 7.0 * zoom / 3.0, Stroke::new(w, Color32::WHITE));
        }
        ElementKind::Current => {
            let (l1, l2) = leads(p1, p2, 28.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            let c = l1.lerp(l2, 0.5);
            let r = l1.distance(l2) * 0.5;
            painter.circle_stroke(
                c,
                r,
                Stroke::new(w, voltage_color((v0 + v1) * 0.5, opts.voltage_range)),
            );
            arrow(
                painter,
                l1.lerp(l2, 0.22),
                l1.lerp(l2, 0.78),
                Color32::WHITE,
                w,
            );
        }
        ElementKind::Diode | ElementKind::Led => {
            let (l1, l2) = leads(p1, p2, 16.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            diode_body(painter, l1, l2, v0, v1, opts.voltage_range, w);
        }
        ElementKind::Zener => {
            let (l1, l2) = leads(p1, p2, 16.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            diode_body(painter, l1, l2, v0, v1, opts.voltage_range, w);
            let dir = (l2 - l1).normalized();
            let n = dir.rot90() * l1.distance(l2) * 0.45;
            painter.line_segment(
                [l2 + n, l2 + n + dir * -6.0 + n.normalized() * 4.0],
                Stroke::new(w, voltage_color(v1, opts.voltage_range)),
            );
        }
        ElementKind::Ground => {
            stroke_grad(painter, p1, p2, 0.0, 0.0, opts.voltage_range, w);
            let dir = (p2 - p1).normalized();
            let n = dir.rot90();
            for i in 0..3 {
                let a = 10.0 - i as f32 * 4.0;
                let along = p2 + dir * (i as f32 * 4.0);
                painter.line_segment(
                    [along - n * a * zoom / 3.0, along + n * a * zoom / 3.0],
                    Stroke::new(w, voltage_color(0.0, opts.voltage_range)),
                );
            }
        }
        ElementKind::Switch => {
            stroke_grad(painter, p1, p2, v0, v1, opts.voltage_range, w * 0.7);
            let dir = (p2 - p1).normalized();
            let n = dir.rot90();
            let open = tag == "open";
            let a = p1.lerp(p2, 0.25);
            let b = if open {
                p1.lerp(p2, 0.75) + n * 10.0 * zoom / 3.0
            } else {
                p1.lerp(p2, 0.75)
            };
            painter.line_segment([a, b], Stroke::new(w + 0.4, Color32::WHITE));
        }
        ElementKind::SwitchSpdt => {
            stroke_grad(painter, p1, p2, v0, v1, opts.voltage_range, w);
            if let Some(&p3) = pts.get(2) {
                let v2 = volts.get(2).copied().unwrap_or(v1);
                stroke_grad(painter, p1, p3, v0, v2, opts.voltage_range, w);
            }
            painter.circle_filled(p1, 3.0, Color32::WHITE);
        }
        ElementKind::LabeledNode => {
            stroke_grad(painter, p1, p2, v0, v0, opts.voltage_range, w);
            painter.circle_stroke(
                p2,
                8.0 * zoom / 3.0,
                Stroke::new(w, voltage_color(v0, opts.voltage_range)),
            );
        }
        ElementKind::Pot => {
            let (l1, l2) = leads(p1, p2, 32.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            zigzag(
                painter,
                l1,
                l2,
                v0,
                v1,
                opts.voltage_range,
                w,
                6.0 * zoom / 3.0,
            );
            if let Some(&wiper) = pts.get(2) {
                let v2 = volts.get(2).copied().unwrap_or(v0);
                let mid = p1.lerp(p2, 0.5);
                stroke_grad(painter, mid, wiper, v2, v2, opts.voltage_range, w);
            }
        }
        ElementKind::Probe | ElementKind::Output | ElementKind::TestPoint => {
            stroke_grad(painter, p1, p2, v0, v1, opts.voltage_range, w);
            painter.circle_stroke(
                p2,
                6.0 * zoom / 3.0,
                Stroke::new(w, voltage_color(v0, opts.voltage_range)),
            );
        }
        ElementKind::Ammeter => {
            stroke_grad(painter, p1, p2, v0, v1, opts.voltage_range, w);
            let c = p1.lerp(p2, 0.5);
            painter.circle_stroke(c, 8.0 * zoom / 3.0, Stroke::new(w, Color32::WHITE));
        }
        ElementKind::OpAmp => {
            let inn = pts[0];
            let inp = pts.get(1).copied().unwrap_or(p2);
            let out = pts.get(2).copied().unwrap_or(p2);
            let v2 = volts.get(2).copied().unwrap_or(v0);
            let body = inn.lerp(inp, 0.5);
            stroke_grad(painter, inn, body, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, inp, body, v1, v1, opts.voltage_range, w);
            stroke_grad(painter, body, out, v2, v2, opts.voltage_range, w);
            let dir = (out - body).normalized();
            let n = dir.rot90() * 14.0 * zoom / 3.0;
            let tip = body.lerp(out, 0.55);
            painter.add(Shape::convex_polygon(
                vec![body + n, body - n, tip],
                Color32::from_rgb(20, 24, 32),
                Stroke::new(w, Color32::from_gray(200)),
            ));
        }
        ElementKind::Vcvs | ElementKind::Vccs => {
            multi_star(painter, pts, volts, opts.voltage_range, w);
        }
        ElementKind::Transistor | ElementKind::Mosfet | ElementKind::Jfet => {
            let base = pts[0];
            let a = pts.get(1).copied().unwrap_or(p2);
            let b = pts.get(2).copied().unwrap_or(p2);
            let va = volts.get(1).copied().unwrap_or(v1);
            let vb = volts.get(2).copied().unwrap_or(v1);
            let body = a.lerp(b, 0.5);
            stroke_grad(painter, base, body, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, a, body, va, va, opts.voltage_range, w);
            stroke_grad(painter, b, body, vb, vb, opts.voltage_range, w);
            let dir = (body - base).normalized();
            let n = dir.rot90() * 8.0 * zoom / 3.0;
            painter.line_segment(
                [body - n, body + n],
                Stroke::new(w + 0.6, Color32::from_gray(210)),
            );
        }
        ElementKind::Ldr | ElementKind::Thermistor | ElementKind::Fuse | ElementKind::Memristor => {
            let (l1, l2) = leads(p1, p2, 32.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            zigzag(
                painter,
                l1,
                l2,
                v0,
                v1,
                opts.voltage_range,
                w,
                6.0 * zoom / 3.0,
            );
        }
        ElementKind::SparkGap | ElementKind::Varactor | ElementKind::Crystal => {
            let (l1, l2) = leads(p1, p2, 10.0 * zoom / 3.0);
            stroke_grad(painter, p1, l1, v0, v0, opts.voltage_range, w);
            stroke_grad(painter, l2, p2, v1, v1, opts.voltage_range, w);
            plate(
                painter,
                l1,
                p2 - p1,
                v0,
                opts.voltage_range,
                w,
                12.0 * zoom / 3.0,
            );
            plate(
                painter,
                l2,
                p2 - p1,
                v1,
                opts.voltage_range,
                w,
                12.0 * zoom / 3.0,
            );
        }
        ElementKind::Schmitt | ElementKind::InvertingSchmitt | ElementKind::Inverter => {
            stroke_grad(painter, p1, p2, v0, v1, opts.voltage_range, w);
        }
        ElementKind::LogicInput => {
            stroke_grad(painter, p1, p2, v0, v0, opts.voltage_range, w);
            painter.circle_filled(p2, 7.0 * zoom / 3.0, voltage_color(v0, opts.voltage_range));
            painter.circle_stroke(p2, 7.0 * zoom / 3.0, Stroke::new(w, Color32::WHITE));
        }
        ElementKind::LogicOutput => {
            stroke_grad(painter, p1, p2, v0, v0, opts.voltage_range, w);
            painter.circle_stroke(
                p2,
                7.0 * zoom / 3.0,
                Stroke::new(w, voltage_color(v0, opts.voltage_range)),
            );
        }
        ElementKind::Sweep
        | ElementKind::AmSource
        | ElementKind::FmSource
        | ElementKind::Antenna => {
            stroke_grad(painter, p1, p2, v0, v0, opts.voltage_range, w);
            painter.circle_filled(p2, 7.0 * zoom / 3.0, voltage_color(v0, opts.voltage_range));
            painter.circle_stroke(p2, 7.0 * zoom / 3.0, Stroke::new(w, Color32::WHITE));
        }
        ElementKind::Transformer
        | ElementKind::TappedTransformer
        | ElementKind::AnalogSwitch
        | ElementKind::AnalogSwitchSpdt
        | ElementKind::Gyrator
        | ElementKind::Relay
        | ElementKind::Ccvs
        | ElementKind::Cccs
        | ElementKind::Ccii
        | ElementKind::AndGate
        | ElementKind::NandGate
        | ElementKind::OrGate
        | ElementKind::NorGate
        | ElementKind::XorGate
        | ElementKind::XnorGate
        | ElementKind::DFlipFlop
        | ElementKind::JkFlipFlop
        | ElementKind::TFlipFlop
        | ElementKind::HalfAdder
        | ElementKind::FullAdder
        | ElementKind::Latch
        | ElementKind::Multiplexer
        | ElementKind::Demultiplexer => {
            multi_star(painter, pts, volts, opts.voltage_range, w);
        }
    }

    if opts.show_values {
        if let Some((val, unit)) = value {
            let mid = p1.lerp(p2, 0.5);
            let n = (p2 - p1).normalized().rot90() * 12.0;
            let mut label = si::si(val, unit);
            if !tag.is_empty() {
                label = format!("{label} {tag}");
            }
            painter.text(
                mid + n,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.0),
                Color32::from_gray(210),
            );
        } else if !tag.is_empty() {
            let mid = p1.lerp(p2, 0.5);
            painter.text(
                mid + (p2 - p1).normalized().rot90() * 12.0,
                egui::Align2::CENTER_CENTER,
                tag,
                egui::FontId::proportional(12.0),
                Color32::from_gray(210),
            );
        }
    }

    if opts.show_currents && current.abs() > 1e-15 && kind != ElementKind::Ground {
        current_dots(painter, p1, p2, current, t, zoom);
    }
}

fn multi_star(painter: &egui::Painter, pts: &[Pos2], volts: &[f64], range: f32, w: f32) {
    if pts.is_empty() {
        return;
    }
    let cx = pts.iter().map(|p| p.x).sum::<f32>() / pts.len() as f32;
    let cy = pts.iter().map(|p| p.y).sum::<f32>() / pts.len() as f32;
    let c = Pos2::new(cx, cy);
    for (i, p) in pts.iter().enumerate() {
        let v = volts.get(i).copied().unwrap_or(0.0);
        stroke_grad(painter, *p, c, v, v, range, w);
    }
    painter.rect_stroke(
        Rect::from_center_size(c, Vec2::splat(18.0)),
        CornerRadius::same(2),
        Stroke::new(w, Color32::from_gray(180)),
        egui::StrokeKind::Inside,
    );
}

fn leads(p1: Pos2, p2: Pos2, body: f32) -> (Pos2, Pos2) {
    let d = p2 - p1;
    let len = d.length().max(1.0);
    let half = (body * 0.5).min(len * 0.4);
    let t = half / len;
    (p1.lerp(p2, 0.5 - t), p1.lerp(p2, 0.5 + t))
}

fn stroke_grad(painter: &egui::Painter, a: Pos2, b: Pos2, va: f64, vb: f64, range: f32, w: f32) {
    let n = 8;
    for i in 0..n {
        let t0 = i as f32 / n as f32;
        let t1 = (i + 1) as f32 / n as f32;
        let v = va + (vb - va) * ((t0 + t1) * 0.5) as f64;
        painter.line_segment(
            [a.lerp(b, t0), a.lerp(b, t1)],
            Stroke::new(w, voltage_color(v, range)),
        );
    }
}

fn zigzag(
    painter: &egui::Painter,
    a: Pos2,
    b: Pos2,
    va: f64,
    vb: f64,
    range: f32,
    w: f32,
    hs: f32,
) {
    let dir = (b - a).normalized();
    let n = dir.rot90() * hs;
    let mut pts = vec![a];
    for i in 0..4 {
        let t_hi = (1.0 + 4.0 * i as f32) / 16.0;
        let t_lo = (3.0 + 4.0 * i as f32) / 16.0;
        pts.push(a.lerp(b, t_hi) + n);
        pts.push(a.lerp(b, t_lo) - n);
    }
    pts.push(b);
    for pair in pts.windows(2) {
        let t = pair[0].distance(a) / a.distance(b).max(1.0);
        let v = va + (vb - va) * t as f64;
        painter.line_segment([pair[0], pair[1]], Stroke::new(w, voltage_color(v, range)));
    }
}

fn plate(painter: &egui::Painter, center: Pos2, along: Vec2, v: f64, range: f32, w: f32, hs: f32) {
    let n = along.normalized().rot90() * hs;
    painter.line_segment(
        [center - n, center + n],
        Stroke::new(w + 0.6, voltage_color(v, range)),
    );
}

fn coils(painter: &egui::Painter, a: Pos2, b: Pos2, va: f64, vb: f64, range: f32, w: f32) {
    let dir = (b - a).normalized();
    let n = dir.rot90();
    let len = a.distance(b);
    let coils = 4;
    let r = len / (coils as f32 * 2.0);
    for c in 0..coils {
        let c0 = a + dir * (r * (1.0 + 2.0 * c as f32));
        let mut pts = Vec::new();
        for k in 0..=10 {
            let ang = std::f32::consts::PI * (k as f32 / 10.0);
            pts.push(c0 + dir * (-r * ang.cos()) + n * (r * ang.sin() * 1.35));
        }
        let t = (c as f32 + 0.5) / coils as f32;
        let v = va + (vb - va) * t as f64;
        painter.add(Shape::line(pts, Stroke::new(w, voltage_color(v, range))));
    }
}

fn diode_body(painter: &egui::Painter, a: Pos2, b: Pos2, va: f64, vb: f64, range: f32, w: f32) {
    let dir = (b - a).normalized();
    let n = dir.rot90() * a.distance(b) * 0.55;
    let tri = vec![a + n, a - n, b];
    painter.add(Shape::convex_polygon(
        tri,
        voltage_color(va, range),
        Stroke::new(w, voltage_color(va, range)),
    ));
    let bar_n = dir.rot90() * a.distance(b) * 0.6;
    painter.line_segment(
        [b - bar_n, b + bar_n],
        Stroke::new(w + 0.8, voltage_color(vb, range)),
    );
}

fn arrow(painter: &egui::Painter, a: Pos2, b: Pos2, color: Color32, w: f32) {
    painter.line_segment([a, b], Stroke::new(w, color));
    let dir = (b - a).normalized();
    let n = dir.rot90();
    let head = 7.0;
    painter.line_segment([b, b - dir * head + n * head * 0.55], Stroke::new(w, color));
    painter.line_segment([b, b - dir * head - n * head * 0.55], Stroke::new(w, color));
}

fn current_dots(painter: &egui::Painter, a: Pos2, b: Pos2, current: f64, t: f64, zoom: f32) {
    let dir = if current >= 0.0 { 1.0 } else { -1.0 };
    let speed = current.abs() * 40.0;
    let phase = (t * speed).rem_euclid(1.0) as f32;
    let r = (2.4 * zoom / 3.0).clamp(2.0, 4.5);
    for k in 0..3 {
        let mut f = phase + k as f32 / 3.0;
        f = f.rem_euclid(1.0);
        let f = if dir < 0.0 { 1.0 - f } else { f };
        painter.circle_filled(a.lerp(b, f), r, Color32::from_rgb(255, 255, 160));
    }
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 < 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

pub fn show_scope(ui: &mut Ui, samples: &[(f64, f64)], ylabel: &str) {
    let (resp, painter) = ui.allocate_painter(
        Vec2::new(ui.available_width(), ui.available_height().max(120.0)),
        Sense::hover(),
    );
    let rect = resp.rect.shrink(8.0);
    painter.rect_filled(
        resp.rect,
        CornerRadius::same(4),
        Color32::from_rgb(12, 14, 18),
    );
    if samples.len() < 2 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Select an element and run to plot V and I",
            egui::FontId::proportional(13.0),
            Color32::from_gray(140),
        );
        return;
    }
    let t0 = samples.first().unwrap().0;
    let t1 = samples.last().unwrap().0;
    let dt = (t1 - t0).max(1e-15);
    let mut ymin = f64::MAX;
    let mut ymax = f64::MIN;
    for &(_, y) in samples {
        ymin = ymin.min(y);
        ymax = ymax.max(y);
    }
    if (ymax - ymin).abs() < 1e-15 {
        ymin -= 1.0;
        ymax += 1.0;
    }
    let pad = (ymax - ymin) * 0.08;
    ymin -= pad;
    ymax += pad;

    let to_pt = |t: f64, y: f64| -> Pos2 {
        let x = rect.left() + ((t - t0) / dt) as f32 * rect.width();
        let yy = rect.bottom() - ((y - ymin) / (ymax - ymin)) as f32 * rect.height();
        Pos2::new(x, yy)
    };
    let mut pts = Vec::with_capacity(samples.len());
    for &(t, y) in samples {
        pts.push(to_pt(t, y));
    }
    painter.add(Shape::line(
        pts,
        Stroke::new(1.6, Color32::from_rgb(80, 200, 255)),
    ));
    painter.text(
        rect.left_top(),
        egui::Align2::LEFT_TOP,
        format!("{ylabel}  {} → {}", si::time(t0), si::time(t1)),
        egui::FontId::proportional(11.0),
        Color32::from_gray(160),
    );
    painter.text(
        rect.left_bottom(),
        egui::Align2::LEFT_BOTTOM,
        si::si(ymin, ""),
        egui::FontId::proportional(10.0),
        Color32::from_gray(130),
    );
    painter.text(
        rect.left_top() + Vec2::new(0.0, 14.0),
        egui::Align2::LEFT_TOP,
        si::si(ymax, ""),
        egui::FontId::proportional(10.0),
        Color32::from_gray(130),
    );
}
