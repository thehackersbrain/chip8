use rand;
use rand::Rng;
use font::FONT_SET;

use CHIP8_HEIGHT;
use CHIP8_WIDTH;
use CHIP8_RAM;

const OPCODE_SIZE: usize = 2;

pub struct OutputState<'a> {
    pub vram: &'a [[u8; CHIP8_WIDTH]; CHIP8_HEIGHT],
    pub vram_changed: bool,
    pub beep: bool,
}

enum ProgramCounter {
    Next,
    Skip,
    Jump(usize),
}

impl ProgramCounter {
    fn skip_if(condition: bool) -> ProgramCounter {
        if condition {
            ProgramCounter::Skip
        } else {
            ProgramCounter::Next
        }
    }
}

pub struct Processor {
    vram: [[u8; CHIP8_WIDTH]; CHIP8_HEIGHT],
    vram_changed: bool,
    ram: [u8; CHIP8_RAM],
    stack: [usize; 16],
    v: [u8; 16],
    i: usize,
    pc: usize,
    sp: usize,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [bool; 16],
    keypad_waiting: bool,
    keypad_register: usize,
}

impl Processor {
    pub fn new() -> Self {

        let mut ram = [0u8; CHIP8_RAM];
        for i in 0..FONT_SET.len() {
            ram[i] = FONT_SET[i];
        }

        Processor {
            vram: [[0; CHIP8_WIDTH]; CHIP8_HEIGHT],
            vram_changed: false,
            ram: ram,
            stack: [0; 16],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; 16],
            keypad_waiting: false,
            keypad_register: 0,
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            let addr = 0x200 + i;
            if addr < 4096 {
                self.ram[0x200 + i] = byte;
            } else {
                break;
            }
        }
    }

    pub fn tick(&mut self, keypad: [bool; 16]) -> OutputState {
        self.keypad = keypad;
        self.vram_changed = false;

        if self.keypad_waiting {
            for i in 0..keypad.len() {
                if keypad[i] {
                    self.keypad_waiting = false;
                    self.v[self.keypad_register] = i as u8;
                    break;
                }
            }
        } else {
            if self.delay_timer > 0 {
                self.delay_timer -= 1
            }
            if self.sound_timer > 0 {
                self.sound_timer -= 1
            }
            let opcode = self.get_opcode();
            self.run_opcode(opcode);
        }

        OutputState {
            vram: &self.vram,
            vram_changed: self.vram_changed,
            beep: self.sound_timer > 0,
        }
    }

    fn get_opcode(&self) -> u16 {
        (self.ram[self.pc] as u16) << 8 | (self.ram[self.pc + 1] as u16)
    }

    fn run_opcode(&mut self, opcode: u16) {
        let nibbles = (
            (opcode & 0xF000) >> 12 as u8,
            (opcode & 0x0F00) >> 8 as u8,
            (opcode & 0x00F0) >> 4 as u8,
            (opcode & 0x000F) as u8,
        );
        let nnn = (opcode & 0x0FFF) as usize;
        let kk = (opcode & 0x00FF) as u8;
        let x = nibbles.1 as usize;
        let y = nibbles.2 as usize;
        let n = nibbles.3 as usize;

        let pc_change = match nibbles { 
            (0x00, 0x00, 0x0e, 0x00) => self.op_00e0(),
            (0x00, 0x00, 0x0e, 0x0e) => self.op_00ee(),
            (0x01, _, _, _) => self.op_1nnn(nnn),
            (0x02, _, _, _) => self.op_2nnn(nnn),
            (0x03, _, _, _) => self.op_3xkk(x, kk),
            (0x04, _, _, _) => self.op_4xkk(x, kk),
            (0x05, _, _, 0x00) => self.op_5xy0(x, y),
            (0x06, _, _, _) => self.op_6xkk(x, kk),
            (0x07, _, _, _) => self.op_7xkk(x, kk),
            (0x08, _, _, 0x00) => self.op_8xy0(x, y),
            (0x08, _, _, 0x01) => self.op_8xy1(x, y),
            (0x08, _, _, 0x02) => self.op_8xy2(x, y),
            (0x08, _, _, 0x03) => self.op_8xy3(x, y),
            (0x08, _, _, 0x04) => self.op_8xy4(x, y),
            (0x08, _, _, 0x05) => self.op_8xy5(x, y),
            (0x08, _, _, 0x06) => self.op_8x06(x),
            (0x08, _, _, 0x07) => self.op_8xy7(x, y),
            (0x08, _, _, 0x0e) => self.op_8x0e(x),
            (0x09, _, _, 0x00) => self.op_9xy0(x, y),
            (0x0a, _, _, _) => self.op_annn(nnn),
            (0x0b, _, _, _) => self.op_bnnn(nnn),
            (0x0c, _, _, _) => self.op_cxkk(x, kk),
            (0x0d, _, _, _) => self.op_dxyn(x, y, n),
            (0x0e, _, 0x09, 0x0e) => self.op_ex9e(x),
            (0x0e, _, 0x0a, 0x01) => self.op_exa1(x),
            (0x0f, _, 0x00, 0x07) => self.op_fx07(x),
            (0x0f, _, 0x00, 0x0a) => self.op_fx0a(x),
            (0x0f, _, 0x01, 0x05) => self.op_fx15(x),
            (0x0f, _, 0x01, 0x08) => self.op_fx18(x),
            (0x0f, _, 0x01, 0x0e) => self.op_fx1e(x),
            (0x0f, _, 0x02, 0x09) => self.op_fx29(x),
            (0x0f, _, 0x03, 0x03) => self.op_fx33(x),
            (0x0f, _, 0x05, 0x05) => self.op_fx55(x),
            (0x0f, _, 0x06, 0x05) => self.op_fx65(x),
            _ => ProgramCounter::Next,
        };

        match pc_change {
            ProgramCounter::Next => self.pc += OPCODE_SIZE,
            ProgramCounter::Skip => self.pc += 2 * OPCODE_SIZE,
            ProgramCounter::Jump(addr) => self.pc = addr,
        }


    }


    // CLS: Clear the display.
    fn op_00e0(&mut self) -> ProgramCounter {
        for y in 0..CHIP8_HEIGHT {
            for x in 0..CHIP8_WIDTH {
                self.vram[y][x] = 0;
            }
        }
        self.vram_changed = true;
        ProgramCounter::Next

    }
    // RET:  Return from a subroutine.
    // The interpreter sets the program counter to the address at the
    // top of the stack, then subtracts 1 from the stack pointer.
    fn op_00ee(&mut self) -> ProgramCounter {
        self.sp -= 1;
        ProgramCounter::Jump(self.stack[self.sp])
    }
    // JP addr
    // The interpreter sets the program counter to nnn.
    fn op_1nnn(&mut self, nnn: usize) -> ProgramCounter {
        ProgramCounter::Jump(nnn)
    }
    // CALL addr
    // The interpreter increments the stack pointer, then puts the
    // current PC on the top of the stack. The PC is then set to nnn.
    fn op_2nnn(&mut self, nnn: usize) -> ProgramCounter {
        self.stack[self.sp] = self.pc + OPCODE_SIZE;
        self.sp += 1;
        ProgramCounter::Jump(nnn)
    }
    // SE Vx, byte:
    // Skip next instruction if Vx = kk.
    fn op_3xkk(&mut self, x: usize, kk: u8) -> ProgramCounter {
        ProgramCounter::skip_if(self.v[x] == kk)
    }
    // SNE Vx, byte.
    // Skip next instruction if Vx != kk.
    fn op_4xkk(&mut self, x: usize, kk: u8) -> ProgramCounter {
        ProgramCounter::skip_if(self.v[x] != kk)
    }
    // SE Vx, Vy
    // Skip next instruction if Vx = Vy.
    fn op_5xy0(&mut self, x: usize, y: usize) -> ProgramCounter {
        ProgramCounter::skip_if(self.v[x] == self.v[y])
    }
    // LD Vx, byte
    // Set Vx = kk.
    fn op_6xkk(&mut self, x: usize, kk: u8) -> ProgramCounter {
        self.v[x] = kk;
        ProgramCounter::Next
    }
    // ADD Vx, byte
    // Set Vx = Vx + kk.
    fn op_7xkk(&mut self, x: usize, kk: u8) -> ProgramCounter {
        let vx = self.v[x] as u16;
        let val = kk as u16;
        let result = vx + val;
        self.v[x] = result as u8;
        ProgramCounter::Next
    }
    // LD Vx, Vy
    // Set Vx = Vy.
    fn op_8xy0(&mut self, x: usize, y: usize) -> ProgramCounter {
        self.v[x] = self.v[y];
        ProgramCounter::Next
    }
    // OR Vx, Vy
    // Set Vx = Vx OR Vy.
    fn op_8xy1(&mut self, x: usize, y: usize) -> ProgramCounter {
        self.v[x] |= self.v[y];
        ProgramCounter::Next
    }
    // AND Vx, Vy
    // Set Vx = Vx AND Vy.
    fn op_8xy2(&mut self, x: usize, y: usize) -> ProgramCounter {
        self.v[x] &= self.v[y];
        ProgramCounter::Next
    }
    // XOR Vx, Vy
    // Set Vx = Vx XOR Vy.
    fn op_8xy3(&mut self, x: usize, y: usize) -> ProgramCounter {
        self.v[x] ^= self.v[y];
        ProgramCounter::Next
    }
    // ADD Vx, Vy
    // The values of Vx and Vy are added together. If the result is
    // greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.
    // Only the lowest 8 bits of the result are kept, and stored in Vx.
    fn op_8xy4(&mut self, x: usize, y: usize) -> ProgramCounter {
        let vx = self.v[x] as u16;
        let vy = self.v[y] as u16;
        let result = vx + vy;
        self.v[x] = result as u8;
        self.v[0x0f] = if result > 0xFF { 1 } else { 0 };
        ProgramCounter::Next
    }
    // SUB Vx, Vy
    // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
    fn op_8xy5(&mut self, x: usize, y: usize) -> ProgramCounter {
        self.v[0x0f] = if self.v[x] > self.v[y] { 1 } else { 0 };
        self.v[x] = self.v[x].wrapping_sub(self.v[y]);
        ProgramCounter::Next
    }
    // SHR Vx {, Vy}
    // If the least-significant bit of Vx is 1, then VF is set to 1,
    // otherwise 0. Then Vx is divided by 2.
    fn op_8x06(&mut self, x: usize) -> ProgramCounter {
        self.v[0x0f] = self.v[x] & 1;
        self.v[x] >>= 1;
        ProgramCounter::Next
    }
    // SUBN Vx, Vy
    // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted
    // from Vy, and the results stored in Vx.
    fn op_8xy7(&mut self, x: usize, y: usize) -> ProgramCounter {
        self.v[0x0f] = if self.v[y] > self.v[x] { 1 } else { 0 };
        self.v[x] = self.v[y].wrapping_sub(self.v[x]);
        ProgramCounter::Next
    }
    // SHL Vx {, Vy}
    // If the most-significant bit of Vx is 1, then VF is set to 1,
    // otherwise to 0. Then Vx is multiplied by 2.
    fn op_8x0e(&mut self, x: usize) -> ProgramCounter {
        self.v[0x0f] = (self.v[x] & 0b10000000) >> 7;
        self.v[x] <<= 1;
        ProgramCounter::Next
    }
    // SNE Vx, Vy
    // Skip next instruction if Vx != Vy.
    fn op_9xy0(&mut self, x: usize, y: usize) -> ProgramCounter {
        ProgramCounter::skip_if(self.v[x] != self.v[y])
    }
    // LD I, addr
    // Set I = nnn.
    fn op_annn(&mut self, nnn: usize) -> ProgramCounter {
        self.i = nnn;
        ProgramCounter::Next
    }
    // JP V0, addr
    // The program counter is set to nnn plus the value of V0.
    fn op_bnnn(&mut self, nnn: usize) -> ProgramCounter {
        ProgramCounter::Jump((self.v[0] as usize) + nnn)
    }
    // RND Vx, byte
    // The interpreter generates a random number from 0 to 255,
    // which is then ANDed with the value kk. The results are stored in Vx.
    fn op_cxkk(&mut self, x: usize, kk: u8) -> ProgramCounter {
        let mut rng = rand::thread_rng();
        self.v[x] = rng.gen::<u8>() & kk;
        ProgramCounter::Next
    }
    // DRW Vx, Vy, n
    // The interpreter reads n bytes from memory, starting at the address
    // stored in I. These bytes are then displayed as sprites on screen at
    // coordinates (Vx, Vy). Sprites are XORed onto the existing screen.
    // If this causes any pixels to be erased, VF is set to 1, otherwise
    // it is set to 0. If the sprite is positioned so part of it is outside
    // the coordinates of the display, it wraps around to the opposite side
    // of the screen.
    fn op_dxyn(&mut self, x: usize, y: usize, n: usize) -> ProgramCounter {
        self.v[0x0f] = 0;
        for byte in 0..n {
            let y = (self.v[y] as usize + byte) % CHIP8_HEIGHT;
            for bit in 0..8 {
                let x = (self.v[x] as usize + bit) % CHIP8_WIDTH;
                let color = (self.ram[self.i + byte] >> (7 - bit)) & 1;
                self.v[0x0f] |= color & self.vram[y][x];
                self.vram[y][x] ^= color;

            }
        }
        self.vram_changed = true;
        ProgramCounter::Next
    }
    // SKP Vx
    // Skip next instruction if key with the value of Vx is pressed.
    fn op_ex9e(&mut self, x: usize) -> ProgramCounter {
        ProgramCounter::skip_if(self.keypad[self.v[x] as usize])
    }
    // SKNP Vx
    // Skip next instruction if key with the value of Vx is NOT pressed.
    fn op_exa1(&mut self, x: usize) -> ProgramCounter {
        ProgramCounter::skip_if(!self.keypad[self.v[x] as usize])
    }
    // LD Vx, DT
    // Set Vx = delay timer value.
    fn op_fx07(&mut self, x: usize) -> ProgramCounter {
        self.v[x] = self.delay_timer;
        ProgramCounter::Next
    }
    // LD Vx, K
    // Wait for a key press, store the value of the key in Vx.
    fn op_fx0a(&mut self, x: usize) -> ProgramCounter {
        self.keypad_waiting = true;
        self.keypad_register = x;
        ProgramCounter::Next
    }
    // LD DT, Vx
    // Set delay timer = Vx.
    fn op_fx15(&mut self, x: usize) -> ProgramCounter {
        self.delay_timer = self.v[x];
        ProgramCounter::Next
    }
    // LD ST, Vx
    // Set sound timer = Vx.
    fn op_fx18(&mut self, x: usize) -> ProgramCounter {
        self.sound_timer = self.v[x];
        ProgramCounter::Next
    }
    // ADD I, Vx
    // Set I = I + Vx
    fn op_fx1e(&mut self, x: usize) -> ProgramCounter {
        self.i += self.v[x] as usize;
        self.v[0x0f] = if self.i > 0x0F00 { 1 } else { 0 };
        ProgramCounter::Next
    }
    // LD F, Vx
    // Set I = location of sprite for digit Vx.
    fn op_fx29(&mut self, x: usize) -> ProgramCounter {
        self.i = (self.v[x] as usize) * 5;
        ProgramCounter::Next
    }

    // LD B, Vx
    // The interpreter takes the decimal value of Vx, and places
    // the hundreds digit in memory at location in I, the tens digit
    // at location I+1, and the ones digit at location I+2.
    fn op_fx33(&mut self, x: usize) -> ProgramCounter {
        self.ram[self.i] = self.v[x] / 100;
        self.ram[self.i + 1] = (self.v[x] % 100) / 10;
        self.ram[self.i + 2] = self.v[x] % 10;
        ProgramCounter::Next
    }

    // LD [I], Vx
    // The interpreter copies the values of registers V0 through Vx
    // into memory, starting at the address in I.
    fn op_fx55(&mut self, x: usize) -> ProgramCounter {
        for i in 0..x + 1 {
            self.ram[self.i + i] = self.v[i];
        }
        ProgramCounter::Next
    }

    // LD Vx, [I]
    // The interpreter reads values from memory starting at location
    // I into registers V0 through Vx.
    fn op_fx65(&mut self, x: usize) -> ProgramCounter {
        for i in 0..x + 1 {
            self.v[i] = self.ram[self.i + i];
        }
        ProgramCounter::Next
    }
}
