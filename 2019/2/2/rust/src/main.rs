use intcode::IntcodeVM;

fn main() {
    let base_vm = IntcodeVM::from_stdin().unwrap();

    for noun in 0..100 {
        for verb in 0..100 {
            let mut vm = base_vm.clone();

            vm.set_memory(1, noun).unwrap();
            vm.set_memory(2, verb).unwrap();

            vm.run_to_end().unwrap();

            if vm.get_memory(0) == Ok(19690720) {
                println!("{}", vm.get_memory(1).unwrap() * 100 + vm.get_memory(2).unwrap());
            }
        }
    }
}
