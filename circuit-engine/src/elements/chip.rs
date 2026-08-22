use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{chip_pin_post, SIDE_E, SIDE_S, SIDE_W};
use crate::ports::Ports;

const FLAG_CUSTOM_VOLTAGE: i32 = 1 << 13;
const DFF_RESET: i32 = 2;
const DFF_SET: i32 = 4;
const DFF_INVERT_SR: i32 = 8;
const JK_RESET: i32 = 2;
const JK_POSITIVE_EDGE: i32 = 4;
const JK_INVERT_RESET: i32 = 8;
const LATCH_STATE: i32 = 2;
const LATCH_NO_EDGE: i32 = 4;
const LATCH_RESET: i32 = 8;
const LATCH_SET: i32 = 16;
const MUX_INVERT_OUT: i32 = 1 << 1;
const MUX_STROBE: i32 = 1 << 2;
const DEMUX_INVERT: i32 = 1 << 4;
const ADDER_BITS: i32 = 2;

#[derive(Clone, Debug)]
struct ChipPin {
    pos: i32,
    side: i32,
    output: bool,
    state: bool,
    value: bool,
    vs: usize,
    current: f64,
}

impl ChipPin {
    fn inn(pos: i32, side: i32) -> Self {
        Self {
            pos,
            side,
            output: false,
            state: false,
            value: false,
            vs: 0,
            current: 0.0,
        }
    }
    fn out(pos: i32, side: i32, state: bool) -> Self {
        Self {
            pos,
            side,
            output: true,
            state,
            value: false,
            vs: 0,
            current: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
enum ChipLogic {
    DFlipFlop,
    JkFlipFlop,
    TFlipFlop,
    HalfAdder,
    FullAdder {
        bits: usize,
        carry_in: usize,
        carry_out: usize,
    },
    Latch {
        bits: usize,
        load_pin: usize,
        reset_pin: Option<usize>,
        set_pin: Option<usize>,
        last_load: bool,
        output_values: Vec<bool>,
    },
    Mux {
        select_bits: usize,
        output_pin: usize,
        select_pin: usize,
        strobe: Option<usize>,
    },
    Demux {
        select_bits: usize,
        input_pin: usize,
        select_pin: usize,
        output_pin: usize,
        output_count: usize,
    },
}

/// ChipElm digital device: each output pin is a voltage source to ground.
pub struct ChipElm {
    ports: Ports,
    pins: Vec<ChipPin>,
    high_voltage: f64,
    last_clock: bool,
    just_loaded: bool,
    kind: ElementKind,
    logic: ChipLogic,
}

impl ChipElm {
    fn assemble(
        p1: (i32, i32),
        flags: i32,
        size_x: i32,
        size_y: i32,
        mut pins: Vec<ChipPin>,
        high_voltage: f64,
        state_volts: &[f64],
        kind: ElementKind,
        logic: ChipLogic,
        just_loaded: bool,
    ) -> Self {
        let posts: Vec<(i32, i32)> = pins
            .iter()
            .map(|p| chip_pin_post(p1, flags, size_x, size_y, p.pos, p.side))
            .collect();
        let mut ports = Ports::many(posts, flags);
        let mut si = 0;
        for (i, pin) in pins.iter_mut().enumerate() {
            if pin.state {
                let v = state_volts.get(si).copied().unwrap_or(0.0);
                si += 1;
                ports.volts[i] = v;
                pin.value = v > high_voltage * 0.5;
            }
        }
        Self {
            ports,
            pins,
            high_voltage,
            last_clock: false,
            just_loaded,
            kind,
            logic,
        }
    }

    pub fn d_flip_flop(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        high_voltage: f64,
        q_volts: f64,
    ) -> Self {
        let _ = (x2, y2);
        let has_set = (flags & DFF_SET) != 0;
        let has_reset = (flags & DFF_RESET) != 0 || has_set;
        let mut pins = vec![
            ChipPin::inn(0, SIDE_W),
            ChipPin::out(0, SIDE_E, true),
            ChipPin::out(if has_set { 1 } else { 2 }, SIDE_E, false),
            ChipPin::inn(1, SIDE_W),
        ];
        if has_set {
            pins.push(ChipPin::inn(2, SIDE_E));
            pins.push(ChipPin::inn(2, SIDE_W));
        } else if has_reset {
            pins.push(ChipPin::inn(2, SIDE_W));
        }
        let mut chip = Self::assemble(
            (x1, y1),
            flags,
            2,
            3,
            pins,
            high_voltage,
            &[q_volts],
            ElementKind::DFlipFlop,
            ChipLogic::DFlipFlop,
            true,
        );
        if chip.pins.len() > 2 {
            chip.pins[2].value = !chip.pins[1].value;
        }
        chip
    }

    pub fn jk_flip_flop(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        high_voltage: f64,
        q_volts: f64,
    ) -> Self {
        let _ = (x2, y2);
        let has_reset = (flags & JK_RESET) != 0;
        let mut pins = vec![
            ChipPin::inn(0, SIDE_W),
            ChipPin::inn(1, SIDE_W),
            ChipPin::inn(2, SIDE_W),
            ChipPin::out(0, SIDE_E, true),
            ChipPin::out(2, SIDE_E, false),
        ];
        if has_reset {
            pins.push(ChipPin::inn(1, SIDE_E));
        }
        let mut chip = Self::assemble(
            (x1, y1),
            flags,
            2,
            3,
            pins,
            high_voltage,
            &[q_volts],
            ElementKind::JkFlipFlop,
            ChipLogic::JkFlipFlop,
            true,
        );
        if chip.pins.len() > 4 {
            chip.pins[4].value = !chip.pins[3].value;
        }
        chip
    }

    pub fn t_flip_flop(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        high_voltage: f64,
        q_volts: f64,
    ) -> Self {
        let _ = (x2, y2);
        let has_set = (flags & DFF_SET) != 0;
        let has_reset = (flags & DFF_RESET) != 0 || has_set;
        let mut pins = vec![
            ChipPin::inn(0, SIDE_W),
            ChipPin::out(0, SIDE_E, true),
            ChipPin::out(if has_set { 1 } else { 2 }, SIDE_E, false),
            ChipPin::inn(1, SIDE_W),
        ];
        if has_set {
            pins.push(ChipPin::inn(2, SIDE_E));
            pins.push(ChipPin::inn(2, SIDE_W));
        } else if has_reset {
            pins.push(ChipPin::inn(2, SIDE_W));
        }
        let mut chip = Self::assemble(
            (x1, y1),
            flags,
            2,
            3,
            pins,
            high_voltage,
            &[q_volts],
            ElementKind::TFlipFlop,
            ChipLogic::TFlipFlop,
            false,
        );
        if chip.pins.len() > 2 {
            chip.pins[2].value = !chip.pins[1].value;
        }
        chip
    }

    pub fn half_adder(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32, high_voltage: f64) -> Self {
        let _ = (x2, y2);
        let pins = vec![
            ChipPin::out(0, SIDE_E, false),
            ChipPin::out(1, SIDE_E, false),
            ChipPin::inn(0, SIDE_W),
            ChipPin::inn(1, SIDE_W),
        ];
        Self::assemble(
            (x1, y1),
            flags,
            2,
            2,
            pins,
            high_voltage,
            &[],
            ElementKind::HalfAdder,
            ChipLogic::HalfAdder,
            false,
        )
    }

    pub fn full_adder(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        bits: usize,
        high_voltage: f64,
    ) -> Self {
        let _ = (x2, y2);
        let bits = bits.max(1);
        let bits_y = bits as i32;
        let size_y = bits_y * 2 + 1;
        let n = bits * 3 + 2;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        bit_pins(&mut pins, bits, 0, SIDE_W, 0, false, false);
        bit_pins(&mut pins, bits, bits_y, SIDE_W, bits, false, false);
        bit_pins(&mut pins, bits, 2, SIDE_E, bits * 2, true, false);
        let carry_in = bits * 3;
        let carry_out = bits * 3 + 1;
        pins[carry_out] = ChipPin::out(0, SIDE_E, false);
        pins[carry_in] = ChipPin::inn(bits_y * 2, SIDE_W);
        Self::assemble(
            (x1, y1),
            flags,
            2,
            size_y,
            pins,
            high_voltage,
            &[],
            ElementKind::FullAdder,
            ChipLogic::FullAdder {
                bits,
                carry_in,
                carry_out,
            },
            false,
        )
    }

    pub fn latch(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        bits: usize,
        high_voltage: f64,
        state_volts: &[f64],
    ) -> Self {
        let _ = (x2, y2);
        let bits = bits.max(2);
        let bits_y = bits as i32;
        let has_reset = (flags & LATCH_RESET) != 0;
        let has_set = (flags & LATCH_SET) != 0;
        let extra_left = (has_reset as usize) + (has_set as usize);
        let size_y = bits_y + 1 + extra_left as i32;
        let n = bits * 2 + 1 + extra_left;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        bit_pins(&mut pins, bits, 0, SIDE_W, 0, false, false);
        bit_pins(
            &mut pins,
            bits,
            0,
            SIDE_E,
            bits,
            true,
            (flags & LATCH_STATE) != 0,
        );
        let mut idx = bits * 2;
        let load_pin = idx;
        pins[idx] = ChipPin::inn(bits_y, SIDE_W);
        idx += 1;
        let mut left_pos = bits_y + 1;
        let reset_pin = if has_reset {
            let p = idx;
            pins[idx] = ChipPin::inn(left_pos, SIDE_W);
            idx += 1;
            left_pos += 1;
            Some(p)
        } else {
            None
        };
        let set_pin = if has_set {
            let p = idx;
            pins[idx] = ChipPin::inn(left_pos, SIDE_W);
            Some(p)
        } else {
            None
        };
        let mut output_values = vec![false; bits];
        for (i, v) in state_volts.iter().take(bits).enumerate() {
            output_values[i] = *v > high_voltage * 0.5;
        }
        Self::assemble(
            (x1, y1),
            flags,
            2,
            size_y,
            pins,
            high_voltage,
            state_volts,
            ElementKind::Latch,
            ChipLogic::Latch {
                bits,
                load_pin,
                reset_pin,
                set_pin,
                last_load: false,
                output_values,
            },
            false,
        )
    }

    pub fn multiplexer(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        select_bits: usize,
        high_voltage: f64,
    ) -> Self {
        let _ = (x2, y2);
        let select_bits = select_bits.clamp(1, 6);
        let output_count = 1usize << select_bits;
        let invert = (flags & MUX_INVERT_OUT) != 0;
        let has_strobe = (flags & MUX_STROBE) != 0;
        let size_x = select_bits as i32 + 1;
        let size_y = output_count as i32 + 1;
        let n = output_count + select_bits + 1 + invert as usize + has_strobe as usize;
        let mut pins = Vec::with_capacity(n);
        for i in 0..output_count {
            pins.push(ChipPin::inn(i as i32, SIDE_W));
        }
        let select_pin = pins.len();
        for i in 0..select_bits {
            pins.push(ChipPin::inn(i as i32 + 1, SIDE_S));
        }
        let output_pin = pins.len();
        pins.push(ChipPin::out(0, SIDE_E, false));
        if invert {
            pins.push(ChipPin::out(1, SIDE_E, false));
        }
        let strobe = if has_strobe {
            let i = pins.len();
            pins.push(ChipPin::inn(0, SIDE_S));
            Some(i)
        } else {
            None
        };
        Self::assemble(
            (x1, y1),
            flags,
            size_x,
            size_y,
            pins,
            high_voltage,
            &[],
            ElementKind::Multiplexer,
            ChipLogic::Mux {
                select_bits,
                output_pin,
                select_pin,
                strobe,
            },
            false,
        )
    }

    pub fn demultiplexer(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        select_bits: usize,
        high_voltage: f64,
    ) -> Self {
        let _ = (x2, y2);
        let select_bits = select_bits.clamp(1, 6);
        let output_count = 1usize << select_bits;
        let size_x = 1 + select_bits as i32;
        let size_y = 1 + output_count as i32;
        let n = 1 + select_bits + output_count;
        let mut pins = Vec::with_capacity(n);
        let output_pin = 0;
        for i in 0..output_count {
            pins.push(ChipPin::out(i as i32, SIDE_E, false));
        }
        let select_pin = pins.len();
        for i in 0..select_bits {
            pins.push(ChipPin::inn(i as i32, SIDE_S));
        }
        let input_pin = pins.len();
        pins.push(ChipPin::inn(0, SIDE_W));
        Self::assemble(
            (x1, y1),
            flags,
            size_x,
            size_y,
            pins,
            high_voltage,
            &[],
            ElementKind::Demultiplexer,
            ChipLogic::Demux {
                select_bits,
                input_pin,
                select_pin,
                output_pin,
                output_count,
            },
            false,
        )
    }

    fn write_output(&mut self, n: usize, value: bool) {
        if n < self.pins.len() {
            self.pins[n].value = value;
        }
    }

    fn pin(&self, n: usize) -> bool {
        self.pins.get(n).is_some_and(|p| p.value)
    }

    fn read_select(&self, select_pin: usize, select_bits: usize) -> usize {
        let mut sel = 0usize;
        for i in 0..select_bits {
            if self.pin(select_pin + i) {
                sel |= 1 << i;
            }
        }
        sel
    }

    fn execute(&mut self) {
        match self.kind {
            ElementKind::DFlipFlop => self.exec_dff(),
            ElementKind::JkFlipFlop => self.exec_jk(),
            ElementKind::TFlipFlop => self.exec_t(),
            ElementKind::HalfAdder => {
                let s = self.pin(2) ^ self.pin(3);
                let c = self.pin(2) && self.pin(3);
                self.write_output(0, s);
                self.write_output(1, c);
            }
            ElementKind::FullAdder => self.exec_full_adder(),
            ElementKind::Latch => self.exec_latch(),
            ElementKind::Multiplexer => self.exec_mux(),
            ElementKind::Demultiplexer => self.exec_demux(),
            _ => {}
        }
    }

    fn exec_dff(&mut self) {
        if self.just_loaded {
            self.just_loaded = false;
            return;
        }
        let flags = self.ports.flags;
        let invert = (flags & DFF_INVERT_SR) != 0;
        let has_set = (flags & DFF_SET) != 0;
        let has_reset = (flags & DFF_RESET) != 0 || has_set;
        let is_set = has_set && self.pins.get(5).is_some_and(|p| p.value != invert);
        let is_reset = has_reset && self.pins.get(4).is_some_and(|p| p.value != invert);
        if is_set || is_reset {
            self.write_output(1, false);
            self.write_output(2, false);
            if is_set {
                self.write_output(1, true);
            }
            if is_reset {
                self.write_output(2, true);
            }
        } else {
            if self.pin(3) && !self.last_clock {
                let d = self.pin(0);
                self.write_output(1, d);
            }
            let q = self.pin(1);
            self.write_output(2, !q);
        }
        self.last_clock = self.pin(3);
    }

    fn exec_jk(&mut self) {
        if self.just_loaded {
            self.just_loaded = false;
            return;
        }
        let flags = self.ports.flags;
        let positive = (flags & JK_POSITIVE_EDGE) != 0;
        let transition = if positive {
            self.pin(1) && !self.last_clock
        } else {
            !self.pin(1) && self.last_clock
        };
        if transition {
            let mut q = self.pin(3);
            if self.pin(0) {
                if self.pin(2) {
                    q = !q;
                } else {
                    q = true;
                }
            } else if self.pin(2) {
                q = false;
            }
            self.write_output(3, q);
        }
        self.last_clock = self.pin(1);
        if (flags & JK_RESET) != 0 {
            let invert = (flags & JK_INVERT_RESET) != 0;
            if self.pins.get(5).is_some_and(|p| p.value != invert) {
                self.write_output(3, false);
            }
        }
        let q = self.pin(3);
        self.write_output(4, !q);
    }

    fn exec_t(&mut self) {
        let flags = self.ports.flags;
        if self.pin(3) && !self.last_clock && self.pin(0) {
            let q = self.pin(1);
            self.write_output(1, !q);
        }
        let has_set = (flags & DFF_SET) != 0;
        let has_reset = (flags & DFF_RESET) != 0 || has_set;
        if has_set && self.pin(5) {
            self.write_output(1, true);
        }
        if has_reset && self.pin(4) {
            self.write_output(1, false);
        }
        let q = self.pin(1);
        self.write_output(2, !q);
        self.last_clock = self.pin(3);
    }

    fn exec_full_adder(&mut self) {
        let ChipLogic::FullAdder {
            bits,
            carry_in,
            carry_out,
        } = self.logic
        else {
            return;
        };
        let mut c = if self.pin(carry_in) { 1 } else { 0 };
        for i in 0..bits {
            let v = (self.pin(i) as u8) + (self.pin(i + bits) as u8) + c;
            c = if v > 1 { 1 } else { 0 };
            self.write_output(i + bits * 2, v & 1 == 1);
        }
        self.write_output(carry_out, c == 1);
    }

    fn exec_latch(&mut self) {
        let flags = self.ports.flags;
        let edge = (flags & LATCH_NO_EDGE) == 0;
        let ChipLogic::Latch {
            bits,
            load_pin,
            reset_pin,
            set_pin,
            last_load,
            ..
        } = self.logic
        else {
            return;
        };
        let load = self.pin(load_pin);
        if let Some(sp) = set_pin {
            if self.pin(sp) {
                if let ChipLogic::Latch {
                    output_values,
                    last_load: ll,
                    ..
                } = &mut self.logic
                {
                    output_values.fill(true);
                    *ll = load;
                }
                self.apply_latch_outputs();
                return;
            }
        }
        if let Some(rp) = reset_pin {
            if self.pin(rp) {
                if let ChipLogic::Latch {
                    output_values,
                    last_load: ll,
                    ..
                } = &mut self.logic
                {
                    output_values.fill(false);
                    *ll = load;
                }
                self.apply_latch_outputs();
                return;
            }
        }
        let should_load = load && (!edge || !last_load);
        if should_load {
            let mut vals = vec![false; bits];
            for i in 0..bits {
                vals[i] = self.pin(i);
            }
            if let ChipLogic::Latch {
                output_values,
                last_load: ll,
                ..
            } = &mut self.logic
            {
                *output_values = vals;
                *ll = load;
            }
        } else if let ChipLogic::Latch { last_load: ll, .. } = &mut self.logic {
            *ll = load;
        }
        self.apply_latch_outputs();
    }

    fn apply_latch_outputs(&mut self) {
        if let ChipLogic::Latch {
            bits,
            output_values,
            ..
        } = &self.logic
        {
            let bits = *bits;
            let vals = output_values.clone();
            for i in 0..bits {
                self.write_output(i + bits, vals[i]);
            }
        }
    }

    fn exec_mux(&mut self) {
        let ChipLogic::Mux {
            select_bits,
            output_pin,
            select_pin,
            strobe,
        } = self.logic
        else {
            return;
        };
        let mut val = self.pin(self.read_select(select_pin, select_bits));
        if strobe.is_some_and(|s| self.pin(s)) {
            val = false;
        }
        self.write_output(output_pin, val);
        if (self.ports.flags & MUX_INVERT_OUT) != 0 {
            self.write_output(output_pin + 1, !val);
        }
    }

    fn exec_demux(&mut self) {
        let ChipLogic::Demux {
            select_bits,
            input_pin,
            select_pin,
            output_pin,
            output_count,
        } = self.logic
        else {
            return;
        };
        let idle = (self.ports.flags & DEMUX_INVERT) != 0;
        let sel = self.read_select(select_pin, select_bits);
        let inp = self.pin(input_pin);
        for i in 0..output_count {
            self.write_output(output_pin + i, idle);
        }
        self.write_output(output_pin + sel, inp);
    }
}

/// MSB-first bit pins, matching ChipElm `makeBitPins` default order.
fn bit_pins(
    pins: &mut [ChipPin],
    count: usize,
    pos: i32,
    side: i32,
    offset: usize,
    output: bool,
    state: bool,
) {
    for i in 0..count {
        let p = pos + (count as i32 - 1 - i as i32);
        pins[offset + i] = if output {
            ChipPin::out(p, side, state)
        } else {
            ChipPin::inn(p, side)
        };
    }
}

impl Element for ChipElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        self.kind
    }
    fn voltage_source_count(&self) -> usize {
        self.pins.iter().filter(|p| p.output).count()
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        self.pins.get(n).is_some_and(|p| p.output)
    }
    fn vs_nodes(&self, local: usize) -> (usize, usize) {
        let mut k = 0;
        for (i, pin) in self.pins.iter().enumerate() {
            if pin.output {
                if k == local {
                    return (0, self.node(i));
                }
                k += 1;
            }
        }
        (0, 0)
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes[post]
    }
    fn set_voltage_source(&mut self, n: usize, vs: usize) {
        let mut k = 0;
        for pin in &mut self.pins {
            if pin.output {
                if k == n {
                    pin.vs = vs;
                    return;
                }
                k += 1;
            }
        }
    }
    fn set_node_voltage(&mut self, n: usize, v: f64) {
        self.ports.set_voltage(n, v);
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.pins
            .iter()
            .find(|p| p.output)
            .map(|p| p.current)
            .unwrap_or(0.0)
    }
    fn set_current(&mut self, local: usize, c: f64) {
        let mut k = 0;
        for pin in &mut self.pins {
            if pin.output {
                if k == local {
                    pin.current = c;
                    if local == 0 {
                        self.ports.current = c;
                    }
                    return;
                }
                k += 1;
            }
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        for (i, pin) in self.pins.iter().enumerate() {
            if pin.output {
                ctx.stamp_voltage_source(0, self.ports.nodes[i], pin.vs, None);
            }
        }
    }
    fn start_iteration(&mut self, _ctx: &mut SimContext) {
        let thr = self.high_voltage * 0.5;
        for i in 0..self.pins.len() {
            if !self.pins[i].output {
                self.pins[i].value = self.ports.volts[i] > thr;
            }
        }
        self.execute();
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        for pin in &self.pins {
            if pin.output {
                ctx.update_voltage_source(pin.vs, if pin.value { self.high_voltage } else { 0.0 });
            }
        }
    }
    fn reset(&mut self) {
        for pin in &mut self.pins {
            pin.value = false;
            pin.current = 0.0;
        }
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
        self.last_clock = false;
        self.just_loaded = false;
        if matches!(self.kind, ElementKind::DFlipFlop | ElementKind::TFlipFlop)
            && self.pins.len() > 2
        {
            self.ports.volts[2] = self.high_voltage;
            self.pins[2].value = true;
        }
        if let ChipLogic::Latch { output_values, .. } = &mut self.logic {
            output_values.fill(false);
        }
    }
}

pub fn parse_chip_high_voltage<'a>(tok: &mut impl Iterator<Item = &'a str>, flags: i32) -> f64 {
    if (flags & FLAG_CUSTOM_VOLTAGE) != 0 {
        tok.next().and_then(|s| s.parse().ok()).unwrap_or(5.0)
    } else {
        5.0
    }
}

pub const FLAG_ADDER_BITS: i32 = ADDER_BITS;
