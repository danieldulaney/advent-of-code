use intcode::IntcodeVM;

fn main() {
    let mut vm = IntcodeVM::from_stdin().unwrap();

    vm.push_input(1);
    vm.run_to_end().unwrap();

    let mut last = 0;
    for output in vm.iter_output() {
        last = *output;
        eprintln!("{}", output);
    }

    println!("{}", last);
}
