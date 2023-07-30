use crate::{
    display::{self, Display},
    Error,
};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::{fmt, ops::Range, time::Duration};
use terminal::{util::Point, Terminal};

const GENERAL_PURPOSE_REGISTER_COUNT: usize = 16;
const MEMORY_SIZE: usize = 0x1000;
const CALL_STACK_RANGE: Range<usize> = 0xEA0..0xEFF;
const START_POINT: u16 = 0x200;

#[derive(Debug)]
pub struct Interpreter {
    /// The program counter, indicating where we are in the program.
    pc: Tribble,
    /// General purpose registers.
    ///
    /// There are 16 registers named V0 to VF. VF is a flag register.
    gpr: [u8; GENERAL_PURPOSE_REGISTER_COUNT],
    /// The address register.
    i: Tribble,
    display: Display,
    /// The stack. It is only used to store return addresses when subroutines are called.
    // TODO: Should it be merged into `memory`?
    stack: Vec<Tribble>,
    /// The available memory.
    memory: [u8; MEMORY_SIZE],
    /// The random number generator.
    rng: SmallRng,
    /// The delay timer. It decrements at a speed of 60 hertz until it reaches 0.
    delay_timer: u8,
    /// The sound timer. It decrements at a speed of 60 hertz until it reaches 0.
    /// If it's not zero, a beeping sound is made.
    sound_timer: u8,
}

impl Interpreter {
    pub fn new(program: Vec<u8>) -> Result<Self, Error> {
        /// Loads the inbuilt 4x5 font into memory.
        fn load_font(memory: &mut [u8; MEMORY_SIZE]) {
            for (i, char) in display::FONT.iter().enumerate() {
                memory[i] = *char;
            }
        }

        let mut memory = [0; MEMORY_SIZE];
        load_font(&mut memory);

        for (i, program_byte) in program.iter().enumerate() {
            if let Some(memory_byte) = memory.get_mut(START_POINT as usize + i) {
                *memory_byte = *program_byte;
            } else {
                return Err(format!("Program is bigger than {} bytes.", MEMORY_SIZE).into());
            }
        }

        Ok(Self {
            pc: Tribble(START_POINT),
            gpr: [0; 16],
            i: Tribble(0x000),
            display: Display::new(),
            stack: Vec::<Tribble>::new(),
            memory,
            rng: SmallRng::from_entropy(),
            delay_timer: 0,
            sound_timer: 0,
        })
    }
}

/// 4 bits.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Nibble(u8);

/// 3 nibbles or 12 bits.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Tribble(u16);

impl fmt::Display for Tribble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:#05X}", self.0))
    }
}

/// Splits the 16 bits into 4 nibbles (one nibble is 4 bits and 4x4 = 16).
fn split_word(word: u16) -> (Nibble, Nibble, Nibble, Nibble) {
    // Zero out the last 3 nibbles at the end of the word,
    // i.e. only keep the first of the 4 nibbles.
    let mut nibbles_to_remove = 3;
    let nibble1 = Nibble((word >> (4 * nibbles_to_remove)) as u8);

    // And now for the rest keep only the relevant nibble with bitwise AND operations. `F` is the nibble to keep.
    // Then with more right shifts the remaining nibbles/zeroes are removed.
    nibbles_to_remove -= 1;
    let nibble2 = Nibble(((word & 0x0F00) >> (4 * nibbles_to_remove)) as u8);
    nibbles_to_remove -= 1;
    let nibble3 = Nibble(((word & 0x00F0) >> (4 * nibbles_to_remove)) as u8);
    nibbles_to_remove -= 1;
    let nibble4 = Nibble(((word & 0x000F) >> (4 * nibbles_to_remove)) as u8);

    (nibble1, nibble2, nibble3, nibble4)
}

impl Tribble {
    fn new(
        nibble1: Nibble,
        nibble2: Nibble,
        nibble3: Nibble, /*byte1: u8, byte2: u8*/
    ) -> Self {
        // let second_nibble = get_second_nibble(byte1).0;

        // // In binary, this adds 8 zeroes to the end, making space for 2 nibbles or 1 byte.
        // let tribble = (second_nibble as u16) << 8;

        // Self(tribble | byte2 as u16)
        Self((((nibble1.0 as u16) << 4) | (nibble2.0 as u16)) << 4 | (nibble3.0 as u16))
    }
}

const CLOCK_HERTZ: f64 = 60.0;
const INPUT_TIMEOUT: Duration = Duration::from_millis(((1.0 / CLOCK_HERTZ) * 1000.0 + 0.5) as u64);

impl Interpreter {
    /// Fetches two bytes (making up one instruction) from the binary.
    ///
    /// Returns `None` if the end of the program has been reached.
    fn get_bytes(&self) -> Option<(u8, u8)> {
        let byte1 = self.memory.get(self.pc.0 as usize)?;
        let byte2 = self.memory.get(self.pc.0 as usize + 1)?;

        Some((*byte1, *byte2))
    }

    fn debug(&self, terminal: &mut Terminal, message: &str) {
        terminal.reset_cursor();
        for _ in 0..terminal.size.width {
            terminal.write(" ");
        }
        terminal.reset_cursor();
        terminal.write(message);
        terminal.flush();
        crate::read_event(terminal);
    }

    fn update_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;

            if self.sound_timer == 0 {
                // todo!("beep");
            }
        }
    }

    fn convert_key(key: char) -> Option<u8> {
        match key.to_ascii_lowercase() {
            '1' => Some(0x1),
            '2' => Some(0x2),
            '3' => Some(0x3),
            '4' => Some(0xc),
            'q' => Some(0x4),
            'w' => Some(0x5),
            'e' => Some(0x6),
            'r' => Some(0xd),
            'a' => Some(0x7),
            's' => Some(0x8),
            'd' => Some(0x9),
            'f' => Some(0xe),
            'z' => Some(0xa),
            'x' => Some(0x0),
            'c' => Some(0xb),
            'v' => Some(0xf),
            _ => None,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> Result<(), Error> {
        // self.debug(terminal, "start");
        while let Some((byte1, byte2)) = self.get_bytes() {
            // self.debug(terminal, "get instruction");
            let instruction = Self::get_instruction(byte1, byte2);
            // self.debug(terminal, "split word");
            let (nibble1, nibble2, nibble3, nibble4) = split_word(instruction);
            // self.debug(terminal, "new address tribble");
            let tribble = Tribble::new(nibble2, nibble3, nibble4);
            //  self.debug(terminal, "got address tribble");

            use terminal::event::{Event, Key};

            let key = if let Some(Event::Key(key)) = terminal.poll_event(
                std::time::Duration::from_secs_f64(0.0001), /*INPUT_TIMEOUT*/
            ) {
                match key {
                    Key::Esc => crate::exit(terminal),
                    Key::Char(char) => Self::convert_key(char),
                    _ => None,
                }
            } else {
                None
            };

            let info: &[std::borrow::Cow<'static, str>] = &[
                "".into(), // Reserve space
                format!("Instruction about to execute: {:#06X}", instruction).into(),
                format!("Program counter: {:#06X}", self.pc.0).into(),
                format!(
                    "Registers: {}",
                    String::from("[")
                        + &self
                            .gpr
                            .iter()
                            .enumerate()
                            .map(|(index, register)| format!("V{:X}: {:X}", index, register))
                            .collect::<Vec<String>>()
                            .join(", ")
                        + "]"
                )
                .into(),
                format!("Address register (I): {}", self.i).into(),
                format!("Delay timer: {}", self.delay_timer).into(),
                format!("Sound timer: {}", self.sound_timer).into(),
            ];

            // 1218

            //  terminal.clear();
            // terminal.reset_cursor();
            // for line in info {
            //     terminal.write(&line);
            //     terminal.next_line();
            // }
            // terminal.flush();
            // crate::read_event(terminal);
            //self.clear_display(terminal);

            // self.debug(
            //     terminal,
            //     &format!("now going into the match, checking {:?}", nibble1),
            // );

            self.next_instruction();

            match nibble1.0 {
                0x0 => match tribble.0 {
                    0x0E0 => {
                        self.clear_display(terminal);
                    }
                    0x0EE => {
                        self.r#return();
                    }
                    _ => {
                        // Exit the interpreter and execute machine code at the given address in memory of the
                        // RCA 1802 for COSMAC VIP.
                        // For that, we would need a COSMAC VIP emulator. Luckily this instruction is mostly unused.
                    }
                },
                0x1 => {
                    self.jump(tribble);
                }
                0x2 => {
                    self.call(tribble);
                }
                0x3 => self.value_equality_skip(nibble2, byte2),
                0x4 => self.value_inequality_skip(nibble2, byte2),
                0x5 => self.register_equality_skip(nibble2, nibble3),
                0x6 => self.set_register_to_value(nibble2, byte2),
                0x7 => self.add_to_register(nibble2, byte2),
                0x8 => match nibble4.0 {
                    0x0 => self.set_registers(nibble2, nibble3),
                    0x1 => self.or_registers(nibble2, nibble3),
                    0x2 => self.and_registers(nibble2, nibble3),
                    0x3 => self.xor_registers(nibble2, nibble3),
                    0x4 => self.add_registers(nibble2, nibble3),
                    0x5 => self.sub_registers1(nibble2, nibble3),
                    0x6 => self.shift_register_right(nibble2),
                    0x7 => self.sub_registers2(nibble2, nibble3),
                    0xE => self.shift_register_left(nibble2),

                    _ => return Err(self.error(byte1, byte2)),
                },
                0x9 => self.register_inequality_skip(nibble2, nibble3),
                0xA => self.set_address_register(tribble),
                0xB => self.jump_with_register(tribble),
                0xC => self.generate_random(nibble2, byte2),
                0xD => self.draw_sprite(terminal, nibble2, nibble3, nibble4),
                0xE => match nibble3.0 {
                    0x9 => self.key_equality_skip(nibble2, key),
                    0xA => self.key_inequality_skip(nibble2, key),
                    _ => return Err(self.error(byte1, byte2)),
                },
                0xF => match byte2 {
                    0x07 => self.get_delay_timer(nibble2),
                    0x0A => self.await_key(terminal, nibble2),
                    0x15 => self.set_delay_timer(nibble2),
                    0x18 => self.set_sound_timer(nibble2),
                    0x1E => self.add_address_register(nibble2),
                    0x29 => self.set_sprite(nibble2),
                    0x33 => self.set_address_register_to_bcd(nibble2),
                    0x55 => self.store_registers(nibble2),
                    0x65 => self.store_memory(nibble2),
                    _ => return Err(self.error(byte1, byte2)),
                },
                _ => {
                    return Err(self.error(byte1, byte2));
                }
            }

            self.update_timers();

            // self.next_instruction();
        }

        Ok(())
    }

    /// Clears the display. (TODO: doesn't need &mut self)
    fn clear_display(&mut self, terminal: &mut Terminal) {
        self.display.clear(terminal);
        // crate::await_fitting_window_width(terminal);
        // let center_x = (terminal.size.width - display::SIZE.width) / 2;
        // crate::await_fitting_window_height(terminal);
        // let center_y = (terminal.size.height - display::SIZE.height) / 2;

        // let center = Self::get_center(terminal);

        // for y in center.y..display::SIZE.height + center.y {
        //     terminal.set_cursor(Point { x: center.x, y });
        //     for _ in 0..display::SIZE.width {
        //         terminal.write("W");
        //     }
        // }
        // terminal.flush();
    }

    /// Returns from a subroutine.
    fn r#return(&mut self) {
        if let Some(address) = self.stack.pop() {
            self.jump(address);
        } else {
            // TODO: keep the error?
            panic!("return outside function");
        }
    }

    /// Go to the given address.
    fn jump(&mut self, address: Tribble) {
        self.pc = address;
        //  self.previous_instruction();
    }

    /// Calls a subroutine at the given address.
    fn call(&mut self, address: Tribble) {
        // Push our current address to the stack so that we can return later.
        self.stack.push(self.pc);
        self.jump(address);
    }

    /// Skips the next instruction if the value of the register is equal to the byte.
    fn value_equality_skip(&mut self, register: Nibble, byte: u8) {
        self.skip_next_instruction_if(self.get_register(register) == byte);
    }

    /// Skips the next instruction if the value of the register is not equal to the byte.
    fn value_inequality_skip(&mut self, register: Nibble, byte: u8) {
        self.skip_next_instruction_if(self.get_register(register) != byte);
    }

    /// Skips the next instruction if the value of the first register is equal to the value of the second register.
    fn register_equality_skip(&mut self, register1: Nibble, register2: Nibble) {
        self.skip_next_instruction_if(self.get_register(register1) == self.get_register(register2));
    }

    /// Sets the register's value to the given one.
    fn set_register_to_value(&mut self, register: Nibble, value: u8) {
        *self.get_mut_register(register) = value;
    }

    /// Adds the value to the register's value.
    fn add_to_register(&mut self, register: Nibble, value: u8) {
        let register = self.get_mut_register(register);

        *register = register.wrapping_add(value);
    }

    /// Sets the first register's value to the one of the second register.
    fn set_registers(&mut self, register1: Nibble, register2: Nibble) {
        *self.get_mut_register(register1) = self.get_register(register2);
    }

    /// ORs the first register's value with the second register's.
    fn or_registers(&mut self, register1: Nibble, register2: Nibble) {
        *self.get_mut_register(register1) |= self.get_register(register2);
    }

    /// ANDs the first register's value with the second register's.
    fn and_registers(&mut self, register1: Nibble, register2: Nibble) {
        *self.get_mut_register(register1) &= self.get_register(register2);
    }

    /// XORs the first register's value with the second register's.
    fn xor_registers(&mut self, register1: Nibble, register2: Nibble) {
        *self.get_mut_register(register1) ^= self.get_register(register2);
    }

    /// Adds the first register's value to the second register's.
    ///
    /// If an overflow occurs, the carry flag is set.
    fn add_registers(&mut self, register1: Nibble, register2: Nibble) {
        let register2_value = self.get_register(register2);
        let register1_value = self.get_mut_register(register1);
        let (result, overflow) = register1_value.overflowing_add(register2_value);
        *register1_value = result;
        if overflow {
            self.set_flag();
        } else {
            self.clear_flag();
        }
    }

    /// Subtracts the second register's value from the first register's.
    ///
    /// If an underflow occurs, the carry flag is set.
    fn sub_registers1(&mut self, register1: Nibble, register2: Nibble) {
        let value2 = self.get_register(register2);
        let value1 = self.get_mut_register(register1);
        let (result, underflow) = value1.overflowing_sub(value2);
        *value1 = result;
        if underflow {
            self.clear_flag();
        } else {
            self.set_flag();
        }
    }

    /// Writes the least significant bit (the last bit) of the given register's value to the flag register and
    /// shifts the register's value to the right by 1.
    fn shift_register_right(&mut self, register: Nibble) {
        let value = self.get_register(register);

        self.store_lsb_in_flag(value);

        *self.get_mut_register(register) >>= 1;
    }

    /// Subtracts the first register's value from the second register's.
    ///
    /// If an underflow occurs, the carry flag is set.
    fn sub_registers2(&mut self, register1: Nibble, register2: Nibble) {
        let value2 = self.get_register(register2);
        let value1 = self.get_mut_register(register1);
        let (result, underflow) = value2.overflowing_sub(*value1);
        *value1 = result;
        if underflow {
            self.clear_flag();
        } else {
            self.set_flag();
        }
    }

    /// Writes the least significant bit (the last bit) of the given register's value to the flag register and
    /// shifts the register's value to the left by 1.
    fn shift_register_left(&mut self, register: Nibble) {
        let value = self.get_register(register);

        self.store_lsb_in_flag(value);

        *self.get_mut_register(register) <<= 1;
    }

    /// Skips the next instruction if the value of the first register is not equal to the value of the second register.
    fn register_inequality_skip(&mut self, register1: Nibble, register2: Nibble) {
        self.skip_next_instruction_if(self.get_register(register1) != self.get_register(register2));
    }

    /// Sets the address register to the given value.
    fn set_address_register(&mut self, address: Tribble) {
        self.i = address;
    }

    /// Adds the register V0 to the given address and jumps to it.
    fn jump_with_register(&mut self, address: Tribble) {
        let address = Tribble((self.get_register(Nibble(0x0)) as u16).wrapping_add(address.0));

        self.jump(address);
    }

    /// Generates a random number in range 0..255, bitwise ANDs it and sets it to the given register's value.
    fn generate_random(&mut self, register: Nibble, byte: u8) {
        let rn = self.rng.gen::<u8>();
        let value = rn & byte;

        // panic!("{}, {:#X}, {}, {:#X}", value, byte, rn, register.0);

        *self.get_mut_register(register) = value;
    }
    // //C201
    // //TODO: In the draw instruction VF is set upon pixel collision.
    // /// Draws the sprite at the given registers' X and Y position with the given height.
    // fn draw_sprite(
    //     &mut self,
    //     terminal: &mut Terminal,
    //     register1: Nibble,
    //     register2: Nibble,
    //     height: Nibble,
    // ) {
    //     // TODO: this is almost certainly wrong
    //     let offset_x = self.get_register(register1);
    //     let offset_y = self.get_register(register2);

    //     // 0xD014
    //     //panic!("{:#X} {:#X} {:#X}", register1.0, register2.0, height.0);

    //     // let center = display::Display::get_center(terminal);

    //     let mut point = Point {
    //         x: offset_x as u16,
    //         y: offset_y as u16,
    //     };

    //     // self.debug(terminal, &format!("{:?}", self.i));

    //     // panic!("{:?}", self.memory);

    //     // assert_eq!(self.memory[self.i.0 as usize], 16);

    //     // panic!(
    //     //     "{:#X} {:#X} {:#X} {} {} {:?}",
    //     //     register1.0, register2.0, height.0, offset_x, offset_y, self.i
    //     // );

    //     //  panic!("{:?}, {:?}", "self.memory", self.memory[self.i.0 as usize]);

    //     // 16

    //     let mut flush_required = false;

    //     for y in 0..=height.0 {
    //         point.y += 1; //y as u16;

    //         let sprite_byte = self.memory[(self.i.0 + y as u16) as usize];

    //         //self.debug(terminal, &format!("{:?}", byte));

    //         let previous_point = point;

    //         //self.debug(terminal, &format!("point: {:?}", point));
    //         point.x += 7;
    //         for index in 0..7 {
    //             let sprite_bit = (sprite_byte >> index) & 1;
    //             //self.debug(terminal, &format!("bit: {:?}, point: {:?}", bit, point));
    //             //if bit == 1 {
    //             //self.display.set(point);
    //             // terminal.set_cursor(point);
    //             // terminal.write("██")
    //             let bit_changed = self.display.xor(terminal, point, sprite_bit == 1);
    //             if bit_changed {
    //                 flush_required = true;
    //                 terminal.set_cursor(Point {
    //                     x: point.x * 2,
    //                     y: point.y * 2 + 10,
    //                 });
    //                 if self.display.get(point) {
    //                     terminal.write("██");
    //                 } else {
    //                     terminal.write("  ");
    //                 }
    //             }
    //             //}
    //             point.x -= 1;
    //         }

    //         assert_eq!(previous_point, point);

    //         // let bits = Bits::new(byte);
    //         // self.debug(terminal, &byte.to_string());
    //         // // Draw the pixels backwards.
    //         // point.x += 7;
    //         // for bit in bits {
    //         //     self.debug(terminal, &bit.to_string());
    //         //     if bit {
    //         //         //self.display.set(point);
    //         //         terminal.set_cursor(point);
    //         //         terminal.write("██")
    //         //     }
    //         //     point.x -= 1;
    //         // }
    //         //assert_eq!(point.x, offset_x as u16, "reduce 8   in `point.x += 8`");
    //     }

    //     if flush_required {
    //         terminal.flush();

    //         // Collision detection
    //         self.set_flag();
    //     }
    //     self.debug(terminal, "end of sprite drawing");
    // }

    fn draw_sprite(
        &mut self,
        terminal: &mut Terminal,
        register1: Nibble,
        register2: Nibble,
        height: Nibble,
    ) {
        let x = self.get_register(register1);
        let y = self.get_register(register2);

        let point = Point {
            x: x as u16,
            y: y as u16,
        };

        let i = self.i.0 as usize;
        let height = height.0 as usize;

        let collision = self
            .display
            .draw_sprite(terminal, point, &self.memory[i..i + height]);

        // TODO: try doing height.0+1
        if collision {
            self.set_flag();
        } else {
            self.clear_flag();
        }

        // let mut point = Point { x: 0, y: 7 };

        // for _ in 0..=height.0 {
        //     // try + 1
        //     point.x += 7;
        //     for index in 0..7 {
        //         let sprite_bit = (sprite_byte >> index) & 1;
        //     }
        // }
    }

    /// Skips the next instruction if a key is pressed and that key is equal to the register's value.
    fn key_equality_skip(&mut self, register: Nibble, key: Option<u8>) {
        if let Some(key) = key {
            let value = self.get_register(register);

            self.skip_next_instruction_if(key == value);
        }
    }

    /// Skips the next instruction if a key is pressed and that key is not equal to the register's value.
    fn key_inequality_skip(&mut self, register: Nibble, key: Option<u8>) {
        if let Some(key) = key {
            let value = self.get_register(register);

            self.skip_next_instruction_if(key != value);
        }
    }

    fn get_delay_timer(&mut self, register: Nibble) {
        *self.get_mut_register(register) = self.delay_timer;
    }

    /// Blocks execution until a key is pressed and stores that key in the given register.
    fn await_key(&mut self, terminal: &mut Terminal, register: Nibble) {
        *self.get_mut_register(register) = Self::await_hex_key(terminal);
    }

    /// Sets the delay timer to the given register's value.
    fn set_delay_timer(&mut self, register: Nibble) {
        self.delay_timer = self.get_register(register);
    }

    /// Sets the sound timer to the given register's value.
    fn set_sound_timer(&mut self, register: Nibble) {
        self.sound_timer = self.get_register(register);
    }

    /// Add the given register's value to the address register.
    fn add_address_register(&mut self, register: Nibble) {
        self.i.0 += self.get_register(register) as u16;
    }

    fn set_sprite(&mut self, register: Nibble) {
        // TODO: this is almost certainly wrong
        self.i.0 = self.get_register(register) as u16;
    }

    /// Stores the BCD (binary-coded decimal) representation of the register's value in the memory of the address register.
    fn set_address_register_to_bcd(&mut self, register: Nibble) {
        let value = self.get_register(register);

        let digit1 = value / 100;
        let digit2 = value / 10 % 10;
        let digit3 = value % 10;

        let i = self.i.0 as usize;
        self.memory[i] = digit1;
        self.memory[i + 1] = digit2;
        self.memory[i + 2] = digit3;
    }

    /// Stores all register values starting from V0 to the given register in memory of the address register.
    fn store_registers(&mut self, register: Nibble) {
        for register in 0..=register.0 {
            let i = (self.i.0 + register as u16) as usize;
            self.memory[i] = self.get_register(Nibble(register));
        }
    }

    /// Fills the registers starting from V0 to the given register with values from memory starting at the address register.
    fn store_memory(&mut self, register: Nibble) {
        for register in 0..=register.0 {
            let i = (self.i.0 + register as u16) as usize;
            *self.get_mut_register(Nibble(register)) = self.memory[i];
        }
    }

    //
    // Utilities
    //

    // /// Polls for a pressed hexadecimal key and returns it unless no key is pressed.
    // fn poll_hex_key(terminal: &mut Terminal) -> Option<u8> {
    //     use terminal::event::{Event, Key};

    //     let key = terminal.poll_event(INPUT_TIMEOUT);

    //     if let Some(Event::Key(Key::Char(char))) = key {
    //         if char.is_ascii_hexdigit() {
    //             Some(char as u8)
    //         } else {
    //             None
    //         }
    //     } else {
    //         None
    //     }
    // }

    /// Blocks execution until a hexadecimal key is pressed and returns it.
    fn await_hex_key(terminal: &mut Terminal) -> u8 {
        use terminal::event::{Event, Key};

        loop {
            let key = crate::read_event(terminal);

            if let Some(Event::Key(Key::Char(char))) = key {
                if let Some(char) = Self::convert_key(char) {
                    return char;
                }
            }
        }
    }

    // TODO: merge this with the normal debugging output and print the error below it
    fn error(&mut self, byte1: u8, byte2: u8) -> Error {
        let instruction = Self::get_instruction(byte1, byte2);

        self.previous_instruction();
        // We are fetching the previous instruction so it can't be the last.
        let (byte1, byte2) = self.get_bytes().unwrap();
        let previous_instruction = Self::get_instruction(byte1, byte2);

        let err = format!(
            "Unknown instruction encountered: {:#X}\n\
             The previous instruction was: {:#X}\n\
             ",
            instruction, previous_instruction
        );
        err.into()
    }

    /// Stores the least significant bit (LSB, the last bit) of the given value into the flag register.
    fn store_lsb_in_flag(&mut self, value: u8) {
        let bit = value & 0b0000_0001;
        self.gpr[0xF] = bit;
    }

    /// Sets the flag.
    fn set_flag(&mut self) {
        self.gpr[0xF] = 1;
    }

    /// Zeroes the flag.
    fn clear_flag(&mut self) {
        self.gpr[0xF] = 0;
    }

    /// Skips the next instruction if the condition is `true`.
    fn skip_next_instruction_if(&mut self, condition: bool) {
        if condition {
            self.next_instruction();
        }
    }

    /// Gets the given register's value.
    fn get_register(&self, register: Nibble) -> u8 {
        self.gpr[register.0 as usize]
    }

    /// Gets a mutable reference to the given register's value.
    fn get_mut_register(&mut self, register: Nibble) -> &mut u8 {
        self.gpr.get_mut(register.0 as usize).unwrap()
    }

    /// Advances the program counter by one instruction.
    fn next_instruction(&mut self) {
        self.pc.0 += 2;
    }

    /// Reverts the program counter by one instruction.
    fn previous_instruction(&mut self) {
        self.pc.0 -= 2;
    }

    fn get_instruction(byte1: u8, byte2: u8) -> u16 {
        // One instruction is stored in two bytes as big-endian.
        // With big endian the bytes are in order and we simply need to put the two bytes together to one 16-bit integer,
        // i.e. we simply concatenate the two bytes.

        // In binary, this adds 8 zeroes to the end of the bits, making it a 16-bit integer (a word).
        // Below we will replace those 8 zeroes with data.
        let word = (byte1 as u16) << 8;

        // And now we simply put the 8 bits of the second byte into those 8 zeroes.
        word | byte2 as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_word() {
        let word = 0xABCD;

        let (nibble1, nibble2, nibble3, nibble4) = split_word(word);

        assert_eq!(nibble1, Nibble(0xA));
        assert_eq!(nibble2, Nibble(0xB));
        assert_eq!(nibble3, Nibble(0xC));
        assert_eq!(nibble4, Nibble(0xD));
    }

    #[test]
    fn test_instruction_fetching() {
        let (byte1, byte2) = (0xAB, 0xFE);
        let instruction = Interpreter::get_instruction(byte1, byte2);
        assert_eq!(instruction, 0xABFE);
        let (nibble1, nibble2, nibble3, nibble4) = split_word(instruction);
        assert_eq!(nibble1, Nibble(0xA));
        assert_eq!(nibble2, Nibble(0xB));
        assert_eq!(nibble3, Nibble(0xF));
        assert_eq!(nibble4, Nibble(0xE));
        let tribble = Tribble::new(nibble2, nibble3, nibble4);
        assert_eq!(tribble, Tribble(0xBFE));
    }
}
