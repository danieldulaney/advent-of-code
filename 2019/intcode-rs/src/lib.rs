use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct IntcodeVM {
    memory: Vec<i64>,
    pc: usize,
    halted: bool,
    input: VecDeque<i64>,
    output: VecDeque<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionError {
    InvalidPC,
    InvalidAddress,
    AlreadyHalted,
    NeedsInput,
    ImmediateModeWrite,
    UnknownOpcode(i64),
    UnknownMode(u8),
}

type Result<T> = std::result::Result<T, ExecutionError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterMode {
    Position,
    Immediate,
}

impl ParameterMode {
    pub fn from_opcode(opcode: i64, parameter_index: u32) -> Result<Self> {
        let place_value = 10i64.pow(parameter_index + 2);

        let mode_value = ((opcode / place_value) % 10) as u8;

        match mode_value {
            0 => Ok(ParameterMode::Position),
            1 => Ok(ParameterMode::Immediate),
            unknown => Err(ExecutionError::UnknownMode(unknown)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    Add(ParameterMode, ParameterMode, ParameterMode),
    Multiply(ParameterMode, ParameterMode, ParameterMode),
    Input(ParameterMode),
    Output(ParameterMode),
    JumpIfTrue(ParameterMode, ParameterMode),
    JumpIfFalse(ParameterMode, ParameterMode),
    LessThan(ParameterMode, ParameterMode, ParameterMode),
    Equals(ParameterMode, ParameterMode, ParameterMode),
    Halt,
}

impl Opcode {
    /// Attempt to parse an opcode from a raw value
    ///
    /// Invalid parameter modes (for example, immediate mode for an output) will
    /// *not* return `Err`, but may cause an error when run.
    pub fn from_raw(raw: i64) -> Result<Self> {
        match raw % 100 {
            1 => Ok(Self::Add(
                ParameterMode::from_opcode(raw, 0)?,
                ParameterMode::from_opcode(raw, 1)?,
                ParameterMode::from_opcode(raw, 2)?,
            )),
            2 => Ok(Self::Multiply(
                ParameterMode::from_opcode(raw, 0)?,
                ParameterMode::from_opcode(raw, 1)?,
                ParameterMode::from_opcode(raw, 2)?,
            )),
            3 => Ok(Self::Input(ParameterMode::from_opcode(raw, 0)?)),
            4 => Ok(Self::Output(ParameterMode::from_opcode(raw, 0)?)),
            5 => Ok(Self::JumpIfTrue(
                ParameterMode::from_opcode(raw, 0)?,
                ParameterMode::from_opcode(raw, 1)?,
            )),
            6 => Ok(Self::JumpIfFalse(
                ParameterMode::from_opcode(raw, 0)?,
                ParameterMode::from_opcode(raw, 1)?,
            )),
            7 => Ok(Self::LessThan(
                ParameterMode::from_opcode(raw, 0)?,
                ParameterMode::from_opcode(raw, 1)?,
                ParameterMode::from_opcode(raw, 2)?,
            )),
            8 => Ok(Self::Equals(
                ParameterMode::from_opcode(raw, 0)?,
                ParameterMode::from_opcode(raw, 1)?,
                ParameterMode::from_opcode(raw, 2)?,
            )),
            99 => Ok(Self::Halt),
            unknown => Err(ExecutionError::UnknownOpcode(unknown)),
        }
    }
}

impl IntcodeVM {
    /// Create a new VM from some existing memory
    pub fn new<D: Into<Vec<i64>>>(data: D) -> Self {
        Self {
            memory: data.into(),
            pc: 0,
            halted: false,
            input: VecDeque::new(),
            output: VecDeque::new(),
        }
    }

    /// Read a comma-separated list of integers from `stdin` and make it into a VM
    pub fn from_stdin() -> std::io::Result<Self> {
        use std::io::prelude::*;

        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;

        let mut data = Vec::new();

        for item in buffer.trim().split(',') {
            let num: i64 = item
                .parse()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, Box::new(e)))?;
            data.push(num)
        }

        Ok(Self::new(data))
    }

    /// Add a single input value to the end of the input queue
    pub fn push_input(&mut self, input: i64) {
        self.input.push_back(input)
    }

    /// Add input values from an interator to the end of the input queue
    pub fn push_inputs<I: IntoIterator<Item = i64>>(&mut self, input: I) {
        self.input.extend(input)
    }

    /// Pop values off the front of the output queue
    pub fn pop_output(&mut self) -> Option<i64> {
        self.output.pop_front()
    }

    /// Get an interator over the output queue
    pub fn iter_output<'a>(&'a self) -> impl Iterator<Item = &i64> {
        self.output.iter()
    }

    /// Get the raw opcode value pointed to by the current PC
    pub fn current_raw_opcode(&self) -> Result<i64> {
        self.get_memory(self.pc)
            .map_err(|_| ExecutionError::InvalidPC)
    }

    /// Get the value of memory at a given index
    pub fn get_memory(&self, index: usize) -> Result<i64> {
        self.memory
            .get(index)
            .map(|v| *v)
            .ok_or(ExecutionError::InvalidPC)
    }

    /// Set the value of memory at a given index
    pub fn set_memory(&mut self, index: usize, value: i64) -> Result<()> {
        self.memory
            .get_mut(index)
            .map(|v| *v = value)
            .ok_or(ExecutionError::InvalidAddress)
    }

    /// Get the value at the memory location pointed to by the value at the given index
    pub fn get_memory_by_pointer(&self, index: usize) -> Result<i64> {
        self.get_memory(Self::value_to_index(self.get_memory(index)?)?)
    }

    /// Set the value at the memory location pointed to by the value at the given index
    pub fn set_memory_by_pointer(&mut self, index: usize, value: i64) -> Result<()> {
        self.set_memory(Self::value_to_index(self.get_memory(index)?)?, value)
    }

    /// Get the parameter based on the given value and the mode
    pub fn get_parameter(&mut self, mode: ParameterMode, offset: usize) -> Result<i64> {
        match mode {
            ParameterMode::Immediate => self.get_memory(self.pc + offset),
            ParameterMode::Position => self.get_memory_by_pointer(self.pc + offset),
        }
    }

    /// Set the parameter based on the given value and the mode
    pub fn set_parameter(&mut self, mode: ParameterMode, offset: usize, value: i64) -> Result<()> {
        match mode {
            ParameterMode::Immediate => Err(ExecutionError::ImmediateModeWrite),
            ParameterMode::Position => self.set_memory_by_pointer(self.pc + offset, value),
        }
    }

    /// Get the entire memory as a slice
    pub fn memory(&self) -> &[i64] {
        &self.memory
    }

    /// Take a single step through the program
    ///
    /// Returns true if the program can continue, or false if the program should
    /// halt.
    ///
    /// If called again on an already halted program, returns `Err(AlreadyHalted)`.
    pub fn step(&mut self) -> Result<bool> {
        let opcode = Opcode::from_raw(self.current_raw_opcode()?)?;

        match &opcode {
            &Opcode::Add(in1, in2, out) => {
                let val1 = self.get_parameter(in1, 1)?;
                let val2 = self.get_parameter(in2, 2)?;
                let result = val1 + val2;
                self.set_parameter(out, 3, result)?;
                self.pc_advance(&opcode);
            }
            &Opcode::Multiply(in1, in2, out) => {
                let val1 = self.get_parameter(in1, 1)?;
                let val2 = self.get_parameter(in2, 2)?;
                let result = val1 * val2;
                self.set_parameter(out, 3, result)?;
                self.pc_advance(&opcode);
            }
            &Opcode::Input(out) => {
                let val = self.input.pop_front().ok_or(ExecutionError::NeedsInput)?;
                self.set_parameter(out, 1, val)?;
                self.pc_advance(&opcode);
            }
            &Opcode::Output(in1) => {
                let val = self.get_parameter(in1, 1)?;
                self.output.push_back(val);
                self.pc_advance(&opcode);
            }
            &Opcode::JumpIfTrue(in1, in2) => {
                let val = self.get_parameter(in1, 1)?;
                let new_loc = self.get_parameter(in2, 2)?;

                if val != 0 {
                    self.pc = Self::value_to_index(new_loc)?;
                } else {
                    self.pc_advance(&opcode);
                }
            }
            &Opcode::JumpIfFalse(in1, in2) => {
                let val = self.get_parameter(in1, 1)?;
                let new_loc = self.get_parameter(in2, 2)?;

                if val == 0 {
                    self.pc = Self::value_to_index(new_loc)?;
                } else {
                    self.pc_advance(&opcode);
                }
            }
            &Opcode::LessThan(in1, in2, out) => {
                let val1 = self.get_parameter(in1, 1)?;
                let val2 = self.get_parameter(in2, 2)?;
                let result = if val1 < val2 { 1 } else { 0 };
                self.set_parameter(out, 3, result)?;
                self.pc_advance(&opcode);
            }
            &Opcode::Equals(in1, in2, out) => {
                let val1 = self.get_parameter(in1, 1)?;
                let val2 = self.get_parameter(in2, 2)?;
                let result = if val1 == val2 { 1 } else { 0 };
                self.set_parameter(out, 3, result)?;
                self.pc_advance(&opcode);
            }
            &Opcode::Halt => self.halted = true,
        };

        Ok(!self.halted)
    }

    /// Run the program until it halts
    pub fn run_to_end(&mut self) -> Result<()> {
        while self.step()? {}

        Ok(())
    }

    /// Check if the VM has halted
    pub fn halted(&self) -> bool {
        self.halted
    }

    fn pc_advance(&mut self, opcode: &Opcode) {
        use Opcode::*;

        let advance = match opcode {
            Add(..) => 4,
            Multiply(..) => 4,
            Input(..) => 2,
            Output(..) => 2,
            JumpIfTrue(..) => 3,
            JumpIfFalse(..) => 3,
            LessThan(..) => 4,
            Equals(..) => 4,
            Halt => 1,
        };

        self.pc += advance;
    }

    fn value_to_index(value: i64) -> Result<usize> {
        use std::convert::TryInto;

        value.try_into().map_err(|_| ExecutionError::InvalidAddress)
    }
}

impl Iterator for IntcodeVM {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        self.output.pop_front()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_parameter_mode() {
        assert_eq!(
            ParameterMode::from_opcode(1002, 0),
            Ok(ParameterMode::Position)
        );
        assert_eq!(
            ParameterMode::from_opcode(1002, 1),
            Ok(ParameterMode::Immediate)
        );
        assert_eq!(
            ParameterMode::from_opcode(1002, 2),
            Ok(ParameterMode::Position)
        );

        assert_eq!(
            ParameterMode::from_opcode(81002, 2),
            Err(ExecutionError::UnknownMode(8))
        );
    }

    #[test]
    fn parse_opcode() {
        use Opcode::*;
        use ParameterMode::*;

        assert_eq!(Opcode::from_raw(1), Ok(Add(Position, Position, Position)));
        assert_eq!(
            Opcode::from_raw(2),
            Ok(Multiply(Position, Position, Position))
        );
        assert_eq!(Opcode::from_raw(3), Ok(Input(Position)));
        assert_eq!(Opcode::from_raw(4), Ok(Output(Position)));
        assert_eq!(Opcode::from_raw(5), Ok(JumpIfTrue(Position, Position)));
        assert_eq!(Opcode::from_raw(6), Ok(JumpIfFalse(Position, Position)));
        assert_eq!(
            Opcode::from_raw(7),
            Ok(LessThan(Position, Position, Position))
        );
        assert_eq!(
            Opcode::from_raw(8),
            Ok(Equals(Position, Position, Position))
        );
        assert_eq!(Opcode::from_raw(99), Ok(Halt));

        assert_eq!(
            Opcode::from_raw(1002),
            Ok(Multiply(Position, Immediate, Position))
        );
        assert_eq!(
            Opcode::from_raw(1101),
            Ok(Add(Immediate, Immediate, Position))
        );

        assert_eq!(Opcode::from_raw(2101), Err(ExecutionError::UnknownMode(2)));

        for opcode in 9..98 {
            assert_eq!(
                Opcode::from_raw(opcode),
                Err(ExecutionError::UnknownOpcode(opcode))
            );
        }
    }

    #[test]
    fn mess_with_memory() {
        let mut vm = IntcodeVM::new(vec![1, 2, 3, 4, 5]);

        assert_eq!(vm.current_raw_opcode(), Ok(1));
        assert_eq!(vm.get_memory(4), Ok(5));
        assert_eq!(vm.set_memory(0, 2), Ok(()));
        assert_eq!(vm.set_memory(4, 12), Ok(()));
        assert_eq!(vm.current_raw_opcode(), Ok(2));
        assert_eq!(vm.get_memory(4), Ok(12));
    }

    #[test]
    fn input_output() {
        let mut single_io = IntcodeVM::new(vec![3, 0, 4, 0, 99]);
        single_io.push_input(5);
        single_io.run_to_end().unwrap();
        assert_eq!(single_io.pop_output(), Some(5));

        let mut flip_io = IntcodeVM::new(vec![3, 0, 3, 1, 4, 1, 4, 0, 99]);
        flip_io.push_inputs([100, 200].iter().copied());
        flip_io.run_to_end().unwrap();
        assert_eq!(flip_io.pop_output(), Some(200));
        assert_eq!(flip_io.pop_output(), Some(100));

        // Take two inputs, sum them, and quadruple them
        let mut quadruple_sum = IntcodeVM::new(vec![
            3, 0, // Input to slot 0
            3, 1, // Input to slot 1
            1, 0, 1, 0, // Sum slots 0 and 1, outputting to slot 0
            102, 4, 0, 0, // Multiply immediate 4 with slot 0, outputting to slot 0
            4, 0, // Output value in slot 0
            99,
        ]);
        quadruple_sum.push_inputs([4, 6].iter().copied());
        quadruple_sum.run_to_end().unwrap();
        assert_eq!(quadruple_sum.pop_output(), Some(40));
    }

    #[test]
    fn opcode_based_on_input() {
        // Take two values and an opcode, then operate on them with that opcode
        // and return the result.
        let program = vec![
            3, 0, // Input to slot 0
            3, 1, // Input to slot 1
            3, 6, // Input to slot 6,
            99, 0, 1,
            2, // Opcode 99 is replaced with 3rd input; runs from 0 and 1, outputting to 2
            4, 2,  // Output from slot 2
            99, // Halt for real this time
        ];

        let mut add_vm = IntcodeVM::new(program.clone());
        add_vm.push_inputs([4, 5, 1].iter().cloned());
        add_vm.run_to_end().unwrap();
        assert_eq!(add_vm.pop_output(), Some(9));

        let mut mul_vm = IntcodeVM::new(program.clone());
        mul_vm.push_inputs([4, 5, 2].iter().cloned());
        mul_vm.run_to_end().unwrap();
        assert_eq!(mul_vm.pop_output(), Some(20));
    }

    #[test]
    fn given_example_day2_1() {
        let mut vm = IntcodeVM::new(vec![1, 9, 10, 3, 2, 3, 11, 0, 99, 30, 40, 50]);

        assert_eq!(vm.step(), Ok(true));

        assert!(!vm.halted());
        assert_eq!(vm.memory(), &[1, 9, 10, 70, 2, 3, 11, 0, 99, 30, 40, 50]);

        assert_eq!(vm.step(), Ok(true));

        assert!(!vm.halted());
        assert_eq!(vm.memory(), &[3500, 9, 10, 70, 2, 3, 11, 0, 99, 30, 40, 50]);

        assert_eq!(vm.step(), Ok(false));
        assert!(vm.halted());
    }

    #[test]
    fn given_example_day2_2() {
        let mut vm1 = IntcodeVM::new(vec![1, 0, 0, 0, 99]);
        let mut vm2 = IntcodeVM::new(vec![2, 3, 0, 3, 99]);
        let mut vm3 = IntcodeVM::new(vec![2, 4, 4, 5, 99, 0]);
        let mut vm4 = IntcodeVM::new(vec![1, 1, 1, 4, 99, 5, 6, 0, 99]);

        vm1.run_to_end().unwrap();
        vm2.run_to_end().unwrap();
        vm3.run_to_end().unwrap();
        vm4.run_to_end().unwrap();

        assert_eq!(vm1.memory(), &[2, 0, 0, 0, 99]);
        assert_eq!(vm2.memory(), &[2, 3, 0, 6, 99]);
        assert_eq!(vm3.memory(), &[2, 4, 4, 5, 99, 9801]);
        assert_eq!(vm4.memory(), &[30, 1, 1, 4, 2, 5, 6, 0, 99]);
    }

    #[test]
    fn given_examples_day5_2_comp() {
        let eq_p = IntcodeVM::new(vec![3, 9, 8, 9, 10, 9, 4, 9, 99, -1, 8]);
        let eq_i = IntcodeVM::new(vec![3, 3, 1108, -1, 8, 3, 4, 3, 99]);
        let lt_p = IntcodeVM::new(vec![3, 9, 7, 9, 10, 9, 4, 9, 99, -1, 8]);
        let lt_i = IntcodeVM::new(vec![3, 3, 1107, -1, 8, 3, 4, 3, 99]);

        {
            let mut eq_p_7 = eq_p.clone();
            eq_p_7.push_input(7);
            eq_p_7.run_to_end().unwrap();
            assert_eq!(eq_p_7.pop_output(), Some(0));

            let mut eq_p_8 = eq_p.clone();
            eq_p_8.push_input(8);
            eq_p_8.run_to_end().unwrap();
            assert_eq!(eq_p_8.pop_output(), Some(1));

            let mut eq_p_9 = eq_p.clone();
            eq_p_9.push_input(9);
            eq_p_9.run_to_end().unwrap();
            assert_eq!(eq_p_9.pop_output(), Some(0));
        }

        {
            let mut lt_i_7 = lt_i.clone();
            lt_i_7.push_input(7);
            lt_i_7.run_to_end().unwrap();
            assert_eq!(lt_i_7.pop_output(), Some(1));

            let mut lt_i_8 = lt_i.clone();
            lt_i_8.push_input(8);
            lt_i_8.run_to_end().unwrap();
            assert_eq!(lt_i_8.pop_output(), Some(0));

            let mut lt_i_9 = lt_i.clone();
            lt_i_9.push_input(9);
            lt_i_9.run_to_end().unwrap();
            assert_eq!(lt_i_9.pop_output(), Some(0));
        }

        {
            let mut lt_p_7 = lt_p.clone();
            lt_p_7.push_input(7);
            lt_p_7.run_to_end().unwrap();
            assert_eq!(lt_p_7.pop_output(), Some(1));

            let mut lt_p_8 = lt_p.clone();
            lt_p_8.push_input(8);
            lt_p_8.run_to_end().unwrap();
            assert_eq!(lt_p_8.pop_output(), Some(0));

            let mut lt_p_9 = lt_p.clone();
            lt_p_9.push_input(9);
            lt_p_9.run_to_end().unwrap();
            assert_eq!(lt_p_9.pop_output(), Some(0));
        }

        {
            let mut eq_i_7 = eq_i.clone();
            eq_i_7.push_input(7);
            eq_i_7.run_to_end().unwrap();
            assert_eq!(eq_i_7.pop_output(), Some(0));

            let mut eq_i_8 = eq_i.clone();
            eq_i_8.push_input(8);
            eq_i_8.run_to_end().unwrap();
            assert_eq!(eq_i_8.pop_output(), Some(1));

            let mut eq_i_9 = eq_i.clone();
            eq_i_9.push_input(9);
            eq_i_9.run_to_end().unwrap();
            assert_eq!(eq_i_9.pop_output(), Some(0));
        }
    }

    #[test]
    fn test_given_examples_day5_2_jump() {
        let pos = IntcodeVM::new(vec![
            3, 12, // Input to position 12
            6, 12, 15, // If position 12 is 0, jump to the position in slot 15 (9)
            1, 13, 14, 13, 4, 13, 99, -1, 0, 1, 9,
        ]);
        let imm = IntcodeVM::new(vec![3, 3, 1105, -1, 9, 1101, 0, 0, 12, 4, 12, 99, 1]);

        let big = IntcodeVM::new(vec![
            3, 21, 1008, 21, 8, 20, 1005, 20, 22, 107, 8, 21, 20, 1006, 20, 31, 1106, 0, 36, 98, 0,
            0, 1002, 21, 125, 20, 4, 20, 1105, 1, 46, 104, 999, 1105, 1, 46, 1101, 1000, 1, 20, 4,
            20, 1105, 1, 46, 98, 99,
        ]);

        {
            let mut pos_0 = pos.clone();
            pos_0.push_input(0);
            pos_0.run_to_end().unwrap();
            assert_eq!(pos_0.pop_output(), Some(0));

            let mut pos_10 = pos.clone();
            pos_10.push_input(10);
            pos_10.run_to_end().unwrap();
            assert_eq!(pos_10.pop_output(), Some(1));
        }

        {
            let mut imm_0 = imm.clone();
            imm_0.push_input(0);
            imm_0.run_to_end().unwrap();
            assert_eq!(imm_0.pop_output(), Some(0));

            let mut imm_10 = imm.clone();
            imm_10.push_input(10);
            imm_10.run_to_end().unwrap();
            assert_eq!(imm_10.pop_output(), Some(1));
        }

        {
            let mut big_7 = big.clone();
            big_7.push_input(7);
            big_7.run_to_end().unwrap();
            assert_eq!(big_7.pop_output(), Some(999));

            let mut big_8 = big.clone();
            big_8.push_input(8);
            big_8.run_to_end().unwrap();
            assert_eq!(big_8.pop_output(), Some(1000));

            let mut big_9 = big.clone();
            big_9.push_input(9);
            big_9.run_to_end().unwrap();
            assert_eq!(big_9.pop_output(), Some(1001));
        }
    }
}
