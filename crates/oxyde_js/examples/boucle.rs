//! Mesure le coût d'une instruction : une boucle vide, et le nombre
//! d'instructions qu'elle exécute par tour.
use std::time::Instant;

fn main() {
    let source = "(function () { for (var i = 0; i < 5000000; i++) {} })()";
    let mut vm = oxyde_js::Vm::new();
    let start = Instant::now();
    let result = vm.eval(source).map(|_| ());
    println!("5 M tours : {:?} ({result:?})", start.elapsed());
    // Plus petit budget qui laisse finir 1000 tours, puis 2000 : la
    // différence donne les instructions par tour.
    let needed = |turns: u32| {
        let source = format!("(function () {{ for (var i = 0; i < {turns}; i++) {{}} }})()");
        (1u64..200_000)
            .find(|budget| {
                let mut vm = oxyde_js::Vm::new();
                vm.set_budget(Some(*budget));
                vm.eval(&source).is_ok()
            })
            .unwrap_or(0)
    };
    let (one, two) = (needed(1000), needed(2000));
    println!(
        "instructions : {one} pour 1000 tours, {two} pour 2000, soit {} par tour",
        (two - one) / 1000
    );
    println!(
        "taille de Value : {} octets",
        std::mem::size_of::<oxyde_js::Value>()
    );
}
