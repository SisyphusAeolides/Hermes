# Hermes chaos scheduling

Hermes uses deterministic nonlinear dynamics for host-side scheduling, not for
security decisions or hardware admission. The equations are kept in
`hermes-core::chaos` and share one `ChaosScheduler` implementation:

- Lorenz (`σ=10`, `ρ=28`, `β=8/3`) and Rössler (`a=.2`, `b=.2`, `c=5.7`)
  provide bounded phase trajectories.
- The logistic map (`x[n+1]=3.99·x[n]·(1−x[n])`) supplies a cheap chaotic
  scalar for decorrelation.
- Duffing (`x'' + δx' + αx + βx³ = γ cos(ωt)`) supplies the driven term.
- The scheduler mixes those trajectories and clamps the result to a 1–50 µs
  service interval; a Lyapunov estimate is retained as a diagnostic.

The scheduler is used at every host-side contention boundary currently in the
stack: lockless DMA-ring acquisition, firmware chunk publication, Falcon
MAILBOX1 polling, and the MPS control broker's command turns. It never changes
the order or digest of firmware bytes, bypasses a hardware gate, or mints an
Online certificate. Hardware protocol timeouts and admission remain ordinary
evidence checks.

Run the deterministic benchmark with:

```sh
cargo run --release -p hermes-ctl -- chaos-benchmark
```

This is a throughput/decorrelation mechanism, not a claim of cryptographic
randomness. Use a cryptographic RNG where unpredictability is required.
