use intcode::IntcodeVM;
use permutohedron::Heap;

fn amplify(base_vm: &IntcodeVM, setting: i64, input: i64) -> i64 {
    let mut vm = base_vm.clone();

    vm.push_input(setting);
    vm.push_input(input);
    vm.run_to_end().unwrap();
    vm.pop_output().unwrap()
}

fn run_arrangement(base_vm: &IntcodeVM, settings: &[i64], mut input: i64) -> i64 {
    for setting in settings {
        input = amplify(base_vm, *setting, input);
    }

    input
}

fn main() {
    let base_vm = IntcodeVM::from_stdin().unwrap();

    let settings = &mut [0, 1, 2, 3, 4];
    let mut max_output = std::i64::MIN;

    for arrangement in Heap::new(settings) {
        let output = run_arrangement(&base_vm, &arrangement, 0);

        eprintln!("{:?} -> {}", arrangement, output);

        if output > max_output {
            max_output = output;
        }
    }

    println!("{}", max_output);
}
