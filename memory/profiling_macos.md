# Profiling on macOS: `sample` + inferno + llvm-cxxfilt

How step 6.1 got flamegraphs of the Rust engine without root:

- `cargo flamegraph` (dtrace) and `py-spy` (task_for_pid) need sudo on macOS. `/usr/bin/sample <pid>`
  does not; `benchmarks/profiling.py` renders in a loop in a child process and samples it.
- Build the extension with names: `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only
  CARGO_PROFILE_RELEASE_STRIP=none uv run maturin build --release`, install the wheel in the env.
- rustc 1.97 emits **v0-mangled** symbols (`_RNv...`). `rustfilt` 0.2 leaves them as is; Xcode's
  `llvm-cxxfilt` (`xcrun --find llvm-cxxfilt`) demangles them. Demangle **frame by frame**: v0 names
  contain `;` (`[Vec<f64>; 6]`), the folded-stack separator, so demangling whole lines splits frames.
- `sample` records every thread every tick, sleeping ones too. Drop stacks ending in a blocking
  wait (`__psynch_cvwait`, `__workq_kernreturn`, ...) before computing shares. `swtch_pri` is rayon
  workers spinning for work: real CPU, keep it, but it is overhead, not work.
- Do not name a script `profile.py`: it shadows the stdlib module that `cProfile` imports.

**Why:** the first attempts gave flamegraphs of mangled names, split frames, and shares diluted
by idle threads.

**How to apply:** rerun `python benchmarks/profiling.py --python <env> --cases ... --outputs ...`
before and after each Phase 6 optimisation and compare the shares, not only the wall times.

Two traps found by the 6.1 bug hunt: a process with cellestial loaded also runs **polars' rayon
pool** (threads named `polars-N`), which spins in `swtch_pri` and once made ggrust look 37% idle;
`profiling.py` drops those threads. Stripping all `<...>` from demangled names turned
`<Type as Trait>::f` into `::f` and merged unrelated impls; strip only generics after a name.
`<deduplicated_symbol>` frames are functions the linker folded together (identical code).
