use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{chip_pin_post, SIDE_E, SIDE_N, SIDE_S, SIDE_W};
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
const CTR_UP_DOWN: i32 = 4;
const CTR_NEG_EDGE: i32 = 8;
const RING_CLOCK_INHIBIT: i32 = 2;
const RING_RESET_HIGH: i32 = 4;
const PISO_NEW: i32 = 2;
const SEQ_PLAY_ONCE: i32 = 4;
const SEQ_RESET: i32 = 8;
const DEC_BLANK: i32 = 1 << 1;
const DEC_BLANK_F: i32 = 1 << 2;
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
    Counter {
        bits: usize,
        invert_reset: bool,
        modulus: i32,
    },
    Counter2 {
        bits: usize,
        modulus: i32,
        clk: usize,
        clr: usize,
        enp: usize,
        ent: usize,
        rco: usize,
        load: usize,
        carry: bool,
    },
    RingCounter {
        bits: usize,
        clock_inhibit: Option<usize>,
    },
    Sipo {
        bits: usize,
        clock_state: bool,
    },
    Piso {
        data: Vec<bool>,
        data_index: i32,
        clock_state: bool,
        load_state: bool,
        data_pin_index: usize,
    },
    SeqGen {
        bit_position: usize,
        bit_count: usize,
        data: Vec<i32>,
        clock_state: bool,
    },
    SevenSegDecoder {
        segment_count: usize,
        has_blank: bool,
        blank_on_f: bool,
    },
    SevenSeg,
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

    pub fn counter(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        bits: usize,
        high_voltage: f64,
        state_volts: &[f64],
        invert_reset: bool,
        modulus: i32,
    ) -> Self {
        let _ = (x2, y2);
        let bits = bits.max(1);
        let has_ud = (flags & CTR_UP_DOWN) != 0;
        let n = bits + 2 + has_ud as usize;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        pins[0] = ChipPin::inn(0, SIDE_W);
        pins[1] = ChipPin::inn(bits as i32 - 1, SIDE_W);
        bit_pins_rev(&mut pins, bits, 0, SIDE_E, 2, true, true);
        if has_ud {
            pins[bits + 2] = ChipPin::inn(bits as i32 - 2, SIDE_W);
        }
        Self::assemble(
            (x1, y1),
            flags,
            2,
            bits as i32,
            pins,
            high_voltage,
            state_volts,
            ElementKind::Counter,
            ChipLogic::Counter {
                bits,
                invert_reset,
                modulus,
            },
            false,
        )
    }

    pub fn counter2(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        bits: usize,
        high_voltage: f64,
        state_volts: &[f64],
        modulus: i32,
    ) -> Self {
        let _ = (x2, y2);
        let bits = bits.max(2);
        let bits_y = bits as i32;
        let n = bits * 2 + 6;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        bit_pins_rev(&mut pins, bits, 1, SIDE_E, 0, true, true);
        bit_pins_rev(&mut pins, bits, 1, SIDE_W, bits, false, false);
        let p = bits * 2;
        pins[p] = ChipPin::inn(0, SIDE_W);
        pins[p + 1] = ChipPin::inn(bits_y + 1, SIDE_W);
        pins[p + 2] = ChipPin::inn(bits_y + 2, SIDE_W);
        pins[p + 3] = ChipPin::out(0, SIDE_E, false);
        pins[p + 4] = ChipPin::inn(bits_y + 1, SIDE_E);
        pins[p + 5] = ChipPin::inn(bits_y + 2, SIDE_E);
        Self::assemble(
            (x1, y1),
            flags,
            2,
            bits_y + 3,
            pins,
            high_voltage,
            state_volts,
            ElementKind::Counter2,
            ChipLogic::Counter2 {
                bits,
                modulus,
                clk: p,
                clr: p + 1,
                enp: p + 2,
                rco: p + 3,
                load: p + 4,
                ent: p + 5,
                carry: false,
            },
            false,
        )
    }

    pub fn ring_counter(
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
        let inhibit = (flags & RING_CLOCK_INHIBIT) != 0 && bits >= 3;
        let n = bits + 2 + inhibit as usize;
        let size_x = bits.max(2) as i32;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        pins[0] = ChipPin::inn(1, SIDE_W);
        pins[1] = ChipPin::inn(size_x - 1, SIDE_S);
        for i in 0..bits {
            pins[i + 2] = ChipPin::out(i as i32, SIDE_N, true);
        }
        let clock_inhibit = if inhibit {
            pins[n - 1] = ChipPin::inn(1, SIDE_S);
            Some(n - 1)
        } else {
            None
        };
        Self::assemble(
            (x1, y1),
            flags,
            size_x,
            2,
            pins,
            high_voltage,
            state_volts,
            ElementKind::RingCounter,
            ChipLogic::RingCounter {
                bits,
                clock_inhibit,
            },
            true,
        )
    }

    pub fn sipo(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        bits: usize,
        high_voltage: f64,
        q_bits: &[bool],
    ) -> Self {
        let _ = (x2, y2);
        let bits = bits.max(1);
        let n = 2 + bits;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        pins[0] = ChipPin::inn(1, SIDE_W);
        pins[1] = ChipPin::inn(2, SIDE_W);
        for i in 0..bits {
            pins[2 + i] = ChipPin::out(i as i32 + 1, SIDE_N, false);
            if q_bits.get(i).copied().unwrap_or(false) {
                pins[2 + i].value = true;
            }
        }
        Self::assemble(
            (x1, y1),
            flags,
            bits as i32 + 1,
            3,
            pins,
            high_voltage,
            &[],
            ElementKind::SipoShift,
            ChipLogic::Sipo {
                bits,
                clock_state: false,
            },
            false,
        )
    }

    pub fn piso(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        bits: usize,
        high_voltage: f64,
        data: Vec<bool>,
    ) -> Self {
        let _ = (x2, y2);
        let bits = bits.max(1);
        let new_bhvr = (flags & PISO_NEW) != 0;
        let n = (if new_bhvr { 4 } else { 3 }) + bits;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        pins[0] = ChipPin::inn(1, SIDE_W);
        pins[1] = ChipPin::inn(2, SIDE_W);
        pins[2] = ChipPin::out(1, SIDE_E, false);
        let data_pin_index = if new_bhvr {
            pins[3] = ChipPin::inn(0, SIDE_W);
            4
        } else {
            3
        };
        for i in 0..bits {
            pins[data_pin_index + i] = ChipPin::inn(bits as i32 - i as i32, SIDE_N);
        }
        let data = if data.len() == bits {
            data
        } else {
            let mut d = vec![false; bits];
            for (i, v) in data.iter().take(bits).enumerate() {
                d[i] = *v;
            }
            d
        };
        if new_bhvr && !data.is_empty() {
            pins[2].value = data[0];
        }
        Self::assemble(
            (x1, y1),
            flags,
            bits as i32 + 2,
            3,
            pins,
            high_voltage,
            &[],
            ElementKind::PisoShift,
            ChipLogic::Piso {
                data,
                data_index: 0,
                clock_state: false,
                load_state: false,
                data_pin_index,
            },
            false,
        )
    }

    pub fn seq_gen(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        high_voltage: f64,
        bit_count: usize,
        data: Vec<i32>,
    ) -> Self {
        let _ = (x2, y2);
        let has_reset = (flags & SEQ_RESET) != 0;
        let n = if has_reset { 3 } else { 2 };
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        pins[0] = ChipPin::inn(0, SIDE_W);
        pins[1] = ChipPin::out(1, SIDE_E, false);
        if has_reset {
            pins[2] = ChipPin::inn(1, SIDE_W);
        }
        let mut bit_count = bit_count;
        if bit_count > data.len() * 32 {
            bit_count = data.len() * 32;
        }
        Self::assemble(
            (x1, y1),
            flags,
            2,
            2,
            pins,
            high_voltage,
            &[],
            ElementKind::SeqGen,
            ChipLogic::SeqGen {
                bit_position: 0,
                bit_count,
                data,
                clock_state: false,
            },
            false,
        )
    }

    pub fn seven_seg_decoder(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        high_voltage: f64,
        segment_type: i32,
    ) -> Self {
        let _ = (x2, y2);
        let segment_count = match segment_type {
            1 => 14,
            2 => 16,
            _ => 7,
        };
        let has_blank = (flags & DEC_BLANK) != 0;
        let n = segment_count + 4 + has_blank as usize;
        let input_pins_y = 4;
        let size_y = segment_count.max(input_pins_y + has_blank as usize) as i32;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); n];
        for i in 0..segment_count {
            pins[i] = ChipPin::out(i as i32, SIDE_E, false);
        }
        bit_pins_rev(&mut pins, 4, 0, SIDE_W, segment_count, false, false);
        if has_blank {
            pins[segment_count + 4] = ChipPin::inn(input_pins_y as i32, SIDE_W);
        }
        Self::assemble(
            (x1, y1),
            flags,
            3,
            size_y,
            pins,
            high_voltage,
            &[],
            ElementKind::SevenSegDecoder,
            ChipLogic::SevenSegDecoder {
                segment_count,
                has_blank,
                blank_on_f: (flags & DEC_BLANK_F) != 0,
            },
            false,
        )
    }

    pub fn seven_seg(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        high_voltage: f64,
        base_segment_count: usize,
        extra_segment: i32,
        diode_direction: i32,
    ) -> Self {
        let _ = (x2, y2);
        let base = match base_segment_count {
            14 => 14,
            16 => 16,
            _ => 7,
        };
        let mut segment_count = base;
        if extra_segment > 0 {
            segment_count += 1;
        }
        let (pin_count, common_pin) = if diode_direction == 0 {
            (segment_count, None)
        } else {
            (segment_count + 1, Some(segment_count))
        };
        let left = (base + 1) / 2;
        let mut size_y = left as i32;
        let size_x = if base == 7 {
            if pin_count > 7 {
                5
            } else {
                4
            }
        } else {
            5
        };
        if pin_count as i32 > size_y * 2 {
            size_y += 1;
        }
        let backward = segment_count == 7 && diode_direction == 0 && extra_segment == 0;
        let mut pins = vec![ChipPin::inn(0, SIDE_W); pin_count];
        for i in 0..left {
            pins[i] = ChipPin::inn(i as i32, SIDE_W);
        }
        let mut s = if backward { 1 } else { 0 };
        let rest_side = if backward { SIDE_S } else { SIDE_E };
        for i in left..segment_count {
            pins[i] = ChipPin::inn(s, rest_side);
            s += 1;
        }
        if let Some(cp) = common_pin {
            let (side, pos) = if segment_count != 7 {
                (SIDE_W, left as i32)
            } else {
                (SIDE_E, s)
            };
            pins[cp] = ChipPin::inn(pos, side);
        }
        Self::assemble(
            (x1, y1),
            flags,
            size_x,
            size_y,
            pins,
            high_voltage,
            &[],
            ElementKind::SevenSeg,
            ChipLogic::SevenSeg,
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
            ElementKind::Counter => self.exec_counter(),
            ElementKind::Counter2 => self.exec_counter2(),
            ElementKind::RingCounter => self.exec_ring(),
            ElementKind::SipoShift => self.exec_sipo(),
            ElementKind::PisoShift => self.exec_piso(),
            ElementKind::SeqGen => self.exec_seqgen(),
            ElementKind::SevenSegDecoder => self.exec_decoder(),
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

    fn exec_counter(&mut self) {
        let flags = self.ports.flags;
        let neg = (flags & CTR_NEG_EDGE) != 0;
        let ChipLogic::Counter {
            bits,
            invert_reset,
            modulus,
        } = self.logic
        else {
            return;
        };
        if self.pin(0) != neg && self.last_clock == neg {
            let dir: i32 = if (flags & CTR_UP_DOWN) != 0 && self.pin(bits + 2) {
                -1
            } else {
                1
            };
            let last_bit = 2 + bits - 1;
            let mut value: i32 = 0;
            for i in 0..bits {
                if self.pin(last_bit - i) {
                    value |= 1 << i;
                }
            }
            value += dir;
            if modulus != 0 {
                value = (value + modulus).rem_euclid(modulus);
            }
            for i in 0..bits {
                self.write_output(last_bit - i, (value & (1 << i)) != 0);
            }
        }
        if !self.pin(1) == invert_reset {
            for i in 0..bits {
                self.write_output(i + 2, false);
            }
        }
        self.last_clock = self.pin(0);
    }

    fn exec_counter2(&mut self) {
        let ChipLogic::Counter2 {
            bits,
            modulus,
            clk,
            clr,
            enp,
            ent,
            rco,
            load,
            carry,
        } = self.logic
        else {
            return;
        };
        let mut carry = carry;
        if self.pin(clk) && !self.last_clock {
            if self.pin(enp) && self.pin(ent) {
                let last_bit = bits - 1;
                let mut value: i32 = 0;
                for i in 0..bits {
                    if self.pin(last_bit - i) {
                        value |= 1 << i;
                    }
                }
                value += 1;
                let realmod = if modulus == 0 { 1 << bits } else { modulus };
                value %= realmod;
                for i in 0..bits {
                    self.write_output(last_bit - i, (value & (1 << i)) != 0);
                }
                carry = value == realmod - 1;
            }
            if !self.pin(load) {
                for i in 0..bits {
                    self.write_output(i, self.pin(i + bits));
                }
                let last_bit = bits - 1;
                let mut value: i32 = 0;
                for i in 0..bits {
                    if self.pin(last_bit - i) {
                        value |= 1 << i;
                    }
                }
                let realmod = if modulus == 0 { 1 << bits } else { modulus };
                carry = value == realmod - 1;
            }
        }
        if !self.pin(clr) {
            for i in 0..bits {
                self.write_output(i, false);
            }
            carry = false;
        }
        self.last_clock = self.pin(clk);
        let rco_val = carry && self.pin(ent);
        self.write_output(rco, rco_val);
        if let ChipLogic::Counter2 { carry: c, .. } = &mut self.logic {
            *c = carry;
        }
    }

    fn exec_ring(&mut self) {
        if self.just_loaded {
            self.just_loaded = false;
            return;
        }
        let invert_reset = (self.ports.flags & RING_RESET_HIGH) == 0;
        let ChipLogic::RingCounter {
            bits,
            clock_inhibit,
        } = self.logic
        else {
            return;
        };
        let running = !clock_inhibit.is_some_and(|p| self.pin(p));
        let mut i = 0;
        while i != bits {
            if self.pin(i + 2) {
                break;
            }
            i += 1;
        }
        if self.pin(0) && !self.last_clock && running {
            if i < bits {
                self.write_output(i + 2, false);
                i += 1;
            }
            i %= bits;
            self.write_output(i + 2, true);
        }
        if self.pin(1) != invert_reset || i == bits {
            for k in 1..bits {
                self.write_output(k + 2, false);
            }
            self.write_output(2, true);
        }
        self.last_clock = self.pin(0);
    }

    fn exec_sipo(&mut self) {
        let ChipLogic::Sipo { bits, clock_state } = self.logic else {
            return;
        };
        let clk = self.pin(1);
        if clk != clock_state {
            if let ChipLogic::Sipo {
                clock_state: cs, ..
            } = &mut self.logic
            {
                *cs = clk;
            }
            if clk && bits > 0 {
                for i in (0..bits.saturating_sub(1)).rev() {
                    let v = self.pin(2 + i);
                    self.write_output(2 + i + 1, v);
                }
                let d = self.pin(0);
                self.write_output(2, d);
            }
        }
    }

    fn exec_piso(&mut self) {
        let new_bhvr = (self.ports.flags & PISO_NEW) != 0;
        let ChipLogic::Piso {
            data_pin_index,
            load_state,
            clock_state,
            ..
        } = self.logic
        else {
            return;
        };
        let ld = self.pin(0);
        if ld != load_state {
            let first = self.pin(data_pin_index);
            let loaded: Vec<bool> = (0..self.pins.len()).map(|i| self.pin(i)).collect();
            if let ChipLogic::Piso {
                data,
                data_index,
                load_state: ls,
                ..
            } = &mut self.logic
            {
                *ls = ld;
                if ld && !data.is_empty() {
                    *data_index = if new_bhvr { 0 } else { -1 };
                    for i in 0..data.len() {
                        data[i] = loaded.get(data_pin_index + i).copied().unwrap_or(false);
                    }
                }
            }
            if ld && new_bhvr {
                self.write_output(2, first);
            }
        }
        let clk = self.pin(1);
        let ser = new_bhvr && self.pin(3);
        if clk != clock_state {
            let mut q = None;
            if let ChipLogic::Piso {
                data,
                data_index,
                clock_state: cs,
                ..
            } = &mut self.logic
            {
                *cs = clk;
                if clk && !data.is_empty() {
                    if *data_index >= 0 {
                        let idx = *data_index as usize;
                        if idx < data.len() {
                            data[idx] = ser;
                        }
                    }
                    *data_index += 1;
                    if *data_index >= data.len() as i32 {
                        *data_index = 0;
                    }
                    let idx = *data_index as usize;
                    q = Some(data[idx]);
                }
            }
            if let Some(v) = q {
                self.write_output(2, v);
            }
        }
    }

    fn exec_seqgen(&mut self) {
        let clk = self.pin(0);
        let rst = (self.ports.flags & SEQ_RESET) != 0 && self.pin(2);
        if rst {
            if let ChipLogic::SeqGen {
                bit_position,
                clock_state,
                ..
            } = &mut self.logic
            {
                *bit_position = 0;
                *clock_state = clk;
            }
            self.next_seq_bit();
            return;
        }
        let edge = matches!(
            &self.logic,
            ChipLogic::SeqGen { clock_state, .. } if clk != *clock_state
        );
        if edge {
            if let ChipLogic::SeqGen {
                clock_state: cs, ..
            } = &mut self.logic
            {
                *cs = clk;
            }
            if clk {
                self.next_seq_bit();
            }
        }
    }

    fn next_seq_bit(&mut self) {
        let play_once = (self.ports.flags & SEQ_PLAY_ONCE) != 0;
        let q = match &mut self.logic {
            ChipLogic::SeqGen {
                bit_position,
                bit_count,
                data,
                ..
            } => {
                if data.is_empty() || *bit_count == 0 {
                    Some(false)
                } else if *bit_position >= *bit_count && play_once {
                    Some(false)
                } else {
                    if *bit_position >= *bit_count {
                        *bit_position = 0;
                    }
                    let idx = *bit_position / 32;
                    let bit = *bit_position % 32;
                    let v = data
                        .get(idx)
                        .is_some_and(|w| ((*w as u32) & (1u32 << bit)) != 0);
                    *bit_position += 1;
                    Some(v)
                }
            }
            _ => None,
        };
        if let Some(v) = q {
            self.write_output(1, v);
        }
    }

    fn exec_decoder(&mut self) {
        let ChipLogic::SevenSegDecoder {
            segment_count,
            has_blank,
            blank_on_f,
        } = self.logic
        else {
            return;
        };
        let mut input = 0usize;
        if self.pin(segment_count) {
            input += 8;
        }
        if self.pin(segment_count + 1) {
            input += 4;
        }
        if self.pin(segment_count + 2) {
            input += 2;
        }
        if self.pin(segment_count + 3) {
            input += 1;
        }
        let en = !(has_blank && !self.pin(segment_count + 4));
        if !en || (input == 15 && blank_on_f) {
            for i in 0..segment_count {
                self.write_output(i, false);
            }
            return;
        }
        for i in 0..segment_count {
            self.write_output(i, decoder_segment(segment_count, input, i));
        }
    }
}

fn decoder_segment(seg_count: usize, digit: usize, i: usize) -> bool {
    let digit = digit.min(15);
    match seg_count {
        14 => SYMBOLS14[digit][i],
        16 => SYMBOLS16[digit][i],
        _ => SYMBOLS7[digit][i],
    }
}

/// a–g for hex digits 0–F.
const SYMBOLS7: [[bool; 7]; 16] = [
    [true, true, true, true, true, true, false],
    [false, true, true, false, false, false, false],
    [true, true, false, true, true, false, true],
    [true, true, true, true, false, false, true],
    [false, true, true, false, false, true, true],
    [true, false, true, true, false, true, true],
    [true, false, true, true, true, true, true],
    [true, true, true, false, false, false, false],
    [true, true, true, true, true, true, true],
    [true, true, true, false, false, true, true],
    [true, true, true, false, true, true, true],
    [false, false, true, true, true, true, true],
    [true, false, false, true, true, true, false],
    [false, true, true, true, true, false, true],
    [true, false, false, true, true, true, true],
    [true, false, false, false, true, true, true],
];

const SYMBOLS14: [[bool; 14]; 16] = [
    [
        true, true, true, true, true, true, false, false, true, false, false, false, true, false,
    ],
    [
        false, true, true, false, false, false, false, false, true, false, false, false, false,
        false,
    ],
    [
        true, true, false, true, true, false, false, false, false, true, false, false, false, true,
    ],
    [
        true, true, true, true, false, false, false, false, false, true, false, false, false, true,
    ],
    [
        false, true, true, false, false, true, false, false, false, true, false, false, false, true,
    ],
    [
        true, false, true, true, false, true, false, false, false, true, false, false, false, true,
    ],
    [
        true, false, true, true, true, true, false, false, false, true, false, false, false, true,
    ],
    [
        true, false, false, false, false, false, false, false, true, false, false, true, false,
        false,
    ],
    [
        true, true, true, true, true, true, false, false, false, true, false, false, false, true,
    ],
    [
        true, true, true, true, false, true, false, false, false, true, false, false, false, true,
    ],
    [
        true, true, true, false, true, true, false, false, false, true, false, false, false, true,
    ],
    [
        true, true, true, true, false, false, false, true, false, true, false, true, false, false,
    ],
    [
        true, false, false, true, true, true, false, false, false, false, false, false, false,
        false,
    ],
    [
        true, true, true, true, false, false, false, true, false, false, false, true, false, false,
    ],
    [
        true, false, false, true, true, true, false, false, false, true, false, false, false, true,
    ],
    [
        true, false, false, false, true, true, false, false, false, true, false, false, false, true,
    ],
];

const SYMBOLS16: [[bool; 16]; 16] = [
    [
        true, true, true, true, true, true, true, true, false, false, true, false, false, false,
        true, false,
    ],
    [
        false, false, true, true, false, false, false, false, false, false, true, false, false,
        false, false, false,
    ],
    [
        true, true, true, false, true, true, true, false, false, false, false, true, false, false,
        false, true,
    ],
    [
        true, true, true, true, true, true, false, false, false, false, false, true, false, false,
        false, true,
    ],
    [
        false, false, true, true, false, false, false, true, false, false, false, true, false,
        false, false, true,
    ],
    [
        true, true, false, true, true, true, false, true, false, false, false, true, false, false,
        false, true,
    ],
    [
        true, true, false, true, true, true, true, true, false, false, false, true, false, false,
        false, true,
    ],
    [
        true, true, false, false, false, false, false, false, false, false, true, false, false,
        true, false, false,
    ],
    [
        true, true, true, true, true, true, true, true, false, false, false, true, false, false,
        false, true,
    ],
    [
        true, true, true, true, true, true, false, true, false, false, false, true, false, false,
        false, true,
    ],
    [
        true, true, true, true, false, false, true, true, false, false, false, true, false, false,
        false, true,
    ],
    [
        true, true, true, true, true, true, false, false, false, true, false, true, false, true,
        false, false,
    ],
    [
        true, true, false, false, true, true, true, true, false, false, false, false, false, false,
        false, false,
    ],
    [
        true, true, true, true, true, true, false, false, false, true, false, false, false, true,
        false, false,
    ],
    [
        true, true, false, false, true, true, true, true, false, false, false, true, false, false,
        false, true,
    ],
    [
        true, true, false, false, false, false, true, true, false, false, false, true, false,
        false, false, true,
    ],
];

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

/// Reversed pin-index order (`makeBitPins(..., reversed=true)`): LSB at `offset+count-1`.
fn bit_pins_rev(
    pins: &mut [ChipPin],
    count: usize,
    pos: i32,
    side: i32,
    offset: usize,
    output: bool,
    state: bool,
) {
    for i in 0..count {
        let ii = offset + count - 1 - i;
        let p = pos + (count as i32 - 1 - i as i32);
        pins[ii] = if output {
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
        if let ChipLogic::Sipo { clock_state, .. } = &mut self.logic {
            *clock_state = false;
        }
        if let ChipLogic::Piso {
            data,
            data_index,
            clock_state,
            load_state,
            ..
        } = &mut self.logic
        {
            data.fill(false);
            *data_index = 0;
            *clock_state = false;
            *load_state = false;
        }
        if let ChipLogic::Counter2 { carry, .. } = &mut self.logic {
            *carry = false;
        }
        if let ChipLogic::SeqGen { bit_position, .. } = &mut self.logic {
            *bit_position = 0;
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
