use intcode::IntcodeVM;

fn main() {
    let mut vm = IntcodeVM::from_stdin().unwrap();

    vm.set_memory(1, 12).unwrap();
    vm.set_memory(2, 2).unwrap();

    vm.run_to_end().unwrap();

    println!("{}", vm.get_memory(0).unwrap());
}
