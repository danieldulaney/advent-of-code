#[derive(Debug, Clone)]
pub struct IntcodeVM {
    memory: Vec<i64>,
    pc: usize,
    halted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionError {
    InvalidPC,
    InvalidAddress,
    AlreadyHalted,
    UnknownOpcode,
}

type Result<T> = std::result::Result<T, ExecutionError>;

impl IntcodeVM {
    pub fn new<D: Into<Vec<i64>>>(data: D) -> Self {
        Self {
            memory: data.into(),
            pc: 0,
            halted: false,
        }
    }

    pub fn current_opcode(&self) -> Result<i64> {
        self.get_memory(self.pc)
            .map_err(|_| ExecutionError::InvalidPC)
    }

    pub fn get_memory(&self, index: usize) -> Result<i64> {
        self.memory
            .get(index)
            .map(|v| *v)
            .ok_or(ExecutionError::InvalidPC)
    }

    pub fn set_memory(&mut self, index: usize, value: i64) -> Result<()> {
        self.memory
            .get_mut(index)
            .map(|v| *v = value)
            .ok_or(ExecutionError::InvalidAddress)
    }

    pub fn get_memory_by_pointer(&self, index: usize) -> Result<i64> {
        self.get_memory(Self::value_to_index(self.get_memory(index)?)?)
    }

    pub fn set_memory_by_pointer(&mut self, index: usize, value: i64) -> Result<()> {
        self.set_memory(Self::value_to_index(self.get_memory(index)?)?, value)
    }

    pub fn memory(&self) -> &[i64] {
        &self.memory
    }

    pub fn step(&mut self) -> Result<bool> {
        let opcode = self.current_opcode()?;

        match self.current_opcode()? {
            1 => self.set_memory_by_pointer(
                self.pc + 3,
                self.get_memory_by_pointer(self.pc + 1)?
                    + self.get_memory_by_pointer(self.pc + 2)?,
            )?,
            2 => self.set_memory_by_pointer(
                self.pc + 3,
                self.get_memory_by_pointer(self.pc + 1)?
                    * self.get_memory_by_pointer(self.pc + 2)?,
            )?,
            99 => self.halted = true,
            _ => return Err(ExecutionError::UnknownOpcode),
        };

        self.pc += Self::pc_advance(opcode);

        Ok(!self.halted)
    }

    pub fn run_to_end(&mut self) -> Result<()> {
        while self.step()? {}

        Ok(())
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    fn pc_advance(opcode: i64) -> usize {
        match opcode {
            1 => 4,
            2 => 4,
            99 => 1,
            _ => panic!("Invalid opcode passed to pc_advance -- this shouldn't ever happen"),
        }
    }

    fn value_to_index(value: i64) -> Result<usize> {
        use std::convert::TryInto;

        value.try_into().map_err(|_| ExecutionError::InvalidAddress)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mess_with_memory() {
        let mut vm = IntcodeVM::new(vec![1, 2, 3, 4, 5]);

        assert_eq!(vm.current_opcode(), Ok(1));
        assert_eq!(vm.get_memory(4), Ok(5));
        assert_eq!(vm.set_memory(0, 2), Ok(()));
        assert_eq!(vm.set_memory(4, 12), Ok(()));
        assert_eq!(vm.current_opcode(), Ok(2));
        assert_eq!(vm.get_memory(4), Ok(12));
    }

    #[test]
    fn unknown_opcodes() {
        for opcode in 3..99 {
            let mut vm = IntcodeVM::new(vec![opcode, 1, 2, 3, 4, 5, 6, 7, 8]);

            eprintln!("Testing opcode {}", opcode);
            assert_eq!(vm.step(), Err(ExecutionError::UnknownOpcode));
        }
    }

    #[test]
    fn given_example_1() {
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
    fn given_example_2() {
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
}
