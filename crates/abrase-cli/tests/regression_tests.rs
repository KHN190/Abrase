use abrase_cli::host::Runtime;
use myriad::Value;
use std::fs;

fn run_example(name: &str) -> Value {
    let path = format!("../../examples/{}", name);
    let src = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));
    let (mut rt, _console) = Runtime::new_for_tests();
    rt.eval(&src).unwrap_or_else(|e| panic!("{} failed:\n{}", name, e))
}

#[test] fn ackermann_runs()      { run_example("ackermann.abe"); }
#[test] fn calc_runs()           { run_example("calc.abe"); }
#[test] fn coin_change_runs()    { run_example("coin_change.abe"); }
#[test] fn lambda_runs()         { run_example("lambda.abe"); }
#[test] fn mandelbrot_runs()     { run_example("mandelbrot.abe"); }
#[test] fn merge_sort_runs()     { run_example("merge_sort.abe"); }
#[test] fn nqueens_runs()        { run_example("nqueens.abe"); }
#[test] fn primes_gen_runs()     { run_example("primes_gen.abe"); }
#[test] fn running_sum_runs()    { run_example("running_sum.abe"); }
#[test] fn stress_dispatch_runs(){ run_example("stress_dispatch.abe"); }
#[test] fn tree_sum_runs()       { run_example("tree_sum.abe"); }

#[test]
fn dual_handler_nested_effects() {
    let v = run_example("dual_handler.abe");
    assert_eq!(v, Value::from_int(205));
}
