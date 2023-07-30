use crate::util::Bits;
use terminal::{
    util::{Point, Size},
    Terminal,
};

pub const SIZE: Size = Size {
    width: 64 + 10,
    height: 32 + 10,
};

/// The display where the graphics are drawn on.
///
/// The display is monochrome and every pixel is either `false` (black) or `true` (white).
#[derive(Debug)]
pub struct Display {
    grid: [[bool; SIZE.width as usize]; SIZE.height as usize],
}

impl Display {
    pub fn new() -> Self {
        Self {
            grid: [[false; SIZE.width as usize]; SIZE.height as usize],
        }
    }

    fn get(&self, point: Point) -> bool {
        self.grid[point.y as usize][point.x as usize]
    }

    fn set(&mut self, point: Point, bit: bool) {
        self.grid[point.y as usize][point.x as usize] = bit;
    }

    fn xor(&mut self, point: Point, bit: bool) {
        self.set(point, self.get(point) ^ bit);
    }

    fn get_center(terminal: &mut Terminal) -> Point {
        crate::await_fitting_window_width(terminal);
        let center_x = (terminal.size.width - SIZE.width) / 2;
        crate::await_fitting_window_height(terminal);
        let center_y = (terminal.size.height - SIZE.height) / 2;

        Point {
            x: center_x,
            y: center_y,
        }
    }

    pub fn clear(&mut self, terminal: &mut Terminal) {
        let center = Self::get_center(terminal);

        for (y, row) in self.grid.iter_mut().enumerate() {
            terminal.set_cursor(Point {
                x: center.x / 2,
                y: center.y + y as u16,
            });
            for bit in row {
                *bit = false;
                terminal.write("W");
            }
        }

        terminal.flush();
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

    /// Draws the sprite and returns whether a any screen pixel is flipped from set to unset.
    pub fn draw_sprite(&mut self, terminal: &mut Terminal, mut point: Point, bytes: &[u8]) -> bool {
        let center = Self::get_center(terminal);

        let mut display_affected = false;
        let mut collision = false;
        for byte in bytes {
            let bits = Bits::new(*byte);

            let previous_point_x = point.x;

            for bit in bits {
                let previous_bit = self.get(point);

                self.xor(point, bit);

                let current_bit = self.get(point);

                if previous_bit && !current_bit {
                    collision = true;
                }

                // terminal.set_cursor(Point {
                //     x: center.x / 2 + point.x * 2,
                //     y: center.y + point.y,
                // });
                // terminal.write("W");

                if current_bit != previous_bit {
                    terminal.set_cursor(Point {
                        x: center.x / 2 + point.x * 2,
                        y: center.y + point.y,
                    });
                    terminal.write("██");
                    display_affected = true;
                }
                point.x += 1;
            }

            point.x = previous_point_x;
            point.y += 1;
        }

        if display_affected {
            terminal.flush();
        }

        collision
    }
}

// The 4x5 inbuilt font.
#[rustfmt::skip]
pub const FONT: [u8; 16 * 7] = [
    // 0
    0b11110000,
    0b10010000,
    0b10010000,
    0b10010000,
    0b11110000,
    0b00000000,
    0b00000000,

    // 1
    0b00110000,
    0b01010000,
    0b10010000,
    0b00010000,
    0b00010000,
    0b00000000,
    0b00000000,

    // 2
    0b01110000,
    0b10010000,
    0b00110000,
    0b01000000,
    0b11110000,
    0b00000000,
    0b00000000,

    // 3
    0b01100000,
    0b10010000,
    0b00110000,
    0b10010000,
    0b01100000,
    0b00000000,
    0b00000000,

    // 4
    0b10010000,
    0b10010000,
    0b11110000,
    0b00010000,
    0b00010000,
    0b00000000,
    0b00000000,

    // 5
    0b11110000,
    0b10000000,
    0b11100000,
    0b00010000,
    0b11100000,
    0b00000000,
    0b00000000,

    // 6
    0b01110000,
    0b10000000,
    0b11100000,
    0b10010000,
    0b01100000,
    0b00000000,
    0b00000000,

    // 7
    0b11110000,
    0b00010000,
    0b00100000,
    0b01000000,
    0b01000000,
    0b00000000,
    0b00000000,

    // 8
    0b01100000,
    0b10010000,
    0b01100000,
    0b10010000,
    0b01100000,
    0b00000000,
    0b00000000,

    // 9
    0b01100000,
    0b10010000,
    0b01110000,
    0b00010000,
    0b01100000,
    0b00000000,
    0b00000000,

    // A
    0b01100000,
    0b10010000,
    0b11110000,
    0b10010000,
    0b10010000,
    0b00000000,
    0b00000000,

    // B
    0b11100000,
    0b10010000,
    0b11100000,
    0b10010000,
    0b11100000,
    0b00000000,
    0b00000000,

    // C
    0b01100000,
    0b10010000,
    0b10000000,
    0b10010000,
    0b01100000,
    0b00000000,
    0b00000000,

    // D
    0b11100000,
    0b10010000,
    0b10010000,
    0b10010000,
    0b11100000,
    0b00000000,
    0b00000000,

    // E
    0b11110000,
    0b10000000,
    0b11110000,
    0b10000000,
    0b11110000,
    0b00000000,
    0b00000000,

    // F
    0b11110000,
    0b10000000,
    0b11110000,
    0b10000000,
    0b10000000,
    0b00000000,
    0b00000000,
];
