use clap::Parser;
use core::{fmt, panic};
use std::{
    error::Error,
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(Copy, Clone)]
enum Instruction {
    Left,
    Right,
    Increment,
    Decrement,
    Output,
    Input,
    BeginLoop,
    EndLoop,
}

impl Instruction {
    fn from_char(a: char) -> Option<Self> {
        return match a {
            '<' => Some(Self::Left),
            '>' => Some(Self::Right),
            '+' => Some(Self::Increment),
            '-' => Some(Self::Decrement),
            '.' => Some(Self::Output),
            ',' => Some(Self::Input),
            '[' => Some(Self::BeginLoop),
            ']' => Some(Self::EndLoop),
            _ => None,
        };
    }
}

struct Program {
    pc: usize,
    source: Vec<Instruction>,
}

#[derive(Debug)]
pub struct MismatchedParen {
    pub message: &'static str,
}

impl fmt::Display for MismatchedParen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for MismatchedParen {}

enum ProgramStepResult {
    Step(Instruction),
    Terminate,
}

impl Program {
    fn parse_source(code: &str) -> Self {
        let source = code.chars().filter_map(Instruction::from_char).collect();
        Self { source, pc: 0 }
    }
    fn execute_command(&mut self, cell_value: u8) -> Result<ProgramStepResult, MismatchedParen> {
        if self.pc >= self.source.len() {
            return Ok(ProgramStepResult::Terminate);
        }
        match &self.source[self.pc] {
            Instruction::BeginLoop => {
                if cell_value == 0 {
                    let mut paren_counter = 0usize;
                    for (i, inst) in self.source[self.pc..self.source.len()].iter().enumerate() {
                        match inst {
                            Instruction::BeginLoop => paren_counter += 1,
                            Instruction::EndLoop => paren_counter -= 1,
                            _ => (),
                        };
                        if paren_counter == 0 {
                            self.pc += i;
                            return Ok(ProgramStepResult::Step(Instruction::BeginLoop));
                        }
                    }
                    Err(MismatchedParen {
                        message: "found no matching closing parenthesis",
                    })
                } else {
                    self.pc += 1;
                    Ok(ProgramStepResult::Step(Instruction::BeginLoop))
                }
            }
            Instruction::EndLoop => {
                if cell_value != 0 {
                    let mut paren_counter = 0usize;
                    for (i, inst) in self.source[0..=self.pc].iter().enumerate().rev() {
                        match inst {
                            Instruction::EndLoop => paren_counter += 1,
                            Instruction::BeginLoop => paren_counter -= 1,
                            _ => (),
                        };
                        if paren_counter == 0 {
                            self.pc = i;
                            return Ok(ProgramStepResult::Step(Instruction::EndLoop));
                        }
                    }
                    Err(MismatchedParen {
                        message: "found no matching opening parenthesis",
                    })
                } else {
                    self.pc += 1;
                    Ok(ProgramStepResult::Step(Instruction::BeginLoop))
                }
            }
            other => {
                self.pc += 1;
                Ok(ProgramStepResult::Step(*other))
            }
        }
    }
}

struct Tape {
    left: Vec<u8>,
    right: Vec<u8>,
    pointer: i64,
    max_size: usize,
}

impl Tape {
    fn new(max_size: usize) -> Self {
        Self {
            left: Vec::new(),
            right: vec![0],
            pointer: 0,
            max_size,
        }
    }
    fn val(&self) -> &u8 {
        if self.pointer < 0 {
            let p: usize = (self.pointer + 1).abs().try_into().unwrap();
            self.left.get(p).unwrap()
        } else {
            let p: usize = self.pointer.try_into().unwrap();
            self.right.get(p).unwrap()
        }
    }
    fn val_mut(&mut self) -> &mut u8 {
        if self.pointer < 0 {
            let p: usize = (self.pointer + 1).abs().try_into().unwrap();
            self.left.get_mut(p).unwrap()
        } else {
            let p: usize = self.pointer.try_into().unwrap();
            self.right.get_mut(p).unwrap()
        }
    }

    fn write(&mut self, c: u8) {
        *self.val_mut() = c;
    }
    fn read(&self) -> u8 {
        *self.val()
    }
    fn increment(&mut self) {
        *self.val_mut() = self.val().wrapping_add(1);
    }
    fn decrement(&mut self) {
        *self.val_mut() = self.val().wrapping_sub(1);
    }
    fn move_left(&mut self) -> Result<(), MaxSizeExceeded> {
        if self.pointer <= 0 && self.left.len() <= self.pointer.abs().try_into().unwrap() {
            if self.left.len() + self.right.len() + 1 > self.max_size {
                return Err(MaxSizeExceeded);
            } else {
                self.left.push(0);
            }
        }
        self.pointer -= 1;
        Ok(())
    }
    fn move_right(&mut self) -> Result<(), MaxSizeExceeded> {
        if self.pointer >= 0 && self.right.len() - 1 <= self.pointer.try_into().unwrap() {
            if self.right.len() + self.left.len() + 1 > self.max_size {
                return Err(MaxSizeExceeded);
            } else {
                self.right.push(0);
            }
        }
        self.pointer += 1;
        Ok(())
    }
}

#[derive(Debug)]
struct MaxSizeExceeded;

impl Error for MaxSizeExceeded {}
impl fmt::Display for MaxSizeExceeded {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "The maximum size specified when creating the tape \
            would be exceededby the operation."
        )
    }
}

struct Runner<I: Fn() -> Result<u8, io::Error>, O: Fn(u8) -> Result<(), io::Error>> {
    tape: Tape,
    program: Program,
    step_count: u64,
    next_halt: Option<u64>,
    input: I,
    output: O,
}

enum RunResult {
    StepCount(u64),
    Terminate,
}

impl<I: Fn() -> Result<u8, io::Error>, O: Fn(u8) -> Result<(), io::Error>> Runner<I, O> {
    fn new(max_size: usize, code: &str, input: I, output: O) -> Self {
        Self {
            tape: Tape::new(max_size),
            program: Program::parse_source(code),
            step_count: 0,
            next_halt: None,
            input,
            output,
        }
    }

    fn step(&mut self) -> Result<ProgramStepResult, Box<dyn Error>> {
        if let ProgramStepResult::Step(instruction) =
            self.program.execute_command(self.tape.read())?
        {
            match instruction {
                Instruction::Left => self.tape.move_left()?,
                Instruction::Right => self.tape.move_right()?,
                Instruction::Increment => {
                    self.tape.increment();
                    ()
                }
                Instruction::Decrement => {
                    self.tape.decrement();
                    ()
                }
                Instruction::Output => (self.output)(self.tape.read())?,
                Instruction::Input => self.tape.write((self.input)()?),
                Instruction::BeginLoop => (),
                Instruction::EndLoop => (),
            }
            self.step_count += 1;
            Ok(ProgramStepResult::Step(instruction))
        } else {
            Ok(ProgramStepResult::Terminate)
        }
    }
    fn run_for(&mut self, steps: u64) -> Result<RunResult, (u64, Box<dyn Error>)> {
        self.next_halt = Some(self.step_count + steps);
        let executed_steps = self.run()?;
        self.next_halt = None;
        Ok(executed_steps)
    }
    fn run(&mut self) -> Result<RunResult, (u64, Box<dyn Error>)> {
        let mut iteration_steps = 0u64;
        loop {
            if let Some(halt_point) = self.next_halt {
                if self.step_count >= halt_point {
                    return Ok(RunResult::StepCount(iteration_steps));
                }
            }

            match self.step() {
                Err(error) => return Err((iteration_steps, error)),
                Ok(res) => match res {
                    ProgramStepResult::Terminate => {
                        return Ok(RunResult::Terminate);
                    }
                    _ => iteration_steps += 1,
                },
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the source file
    #[arg(short, long)]
    source_file: PathBuf,

    /// Maximum size of the tape, default is ~500mb
    #[arg(short, long, default_value_t = 2usize.pow(29))]
    max_size: usize,

    #[arg(short = 'b', long)]
    steps_before_interrupt: Option<u64>,
}

fn main() {
    let args = Args::parse();
    if !args.source_file.exists() {
        panic!("source file {} does not exist", args.source_file.display());
    }
    let source: String = fs::read_to_string(args.source_file).unwrap();
    let mut runner = Runner::new(
        args.max_size,
        &source,
        || {
            let mut byte = [0u8];
            match io::stdin().read_exact(&mut byte) {
                Ok(_) => Ok(byte[0]),
                Err(error) => match error.kind() {
                    io::ErrorKind::UnexpectedEof => Ok(0),
                    _ => Err(error),
                },
            }
        },
        |c| {
            let byte = [c];
            io::stdout().write(&byte).map(|_| ())
        },
    );
    if let Some(steps) = args.steps_before_interrupt {
        loop {
            match runner.run_for(steps).unwrap() {
                RunResult::StepCount(_) => (),
                RunResult::Terminate => {
                    println!("Program halted!");
                    break;
                }
            }
        }
    } else {
        match runner.run().unwrap() {
            RunResult::StepCount(_) => (),
            RunResult::Terminate => {
                println!("Program halted!");
            }
        }
    }
}
