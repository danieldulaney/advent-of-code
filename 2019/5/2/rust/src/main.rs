use intcode::IntcodeVM;

fn main() {
    let mut vm = IntcodeVM::from_stdin().unwrap();

    vm.push_input(5);
    vm.run_to_end().unwrap();

    println!("{}", vm.pop_output().unwrap());
}
