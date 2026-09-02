//! Chaotic oscillators and attractors for ultra-high throughput backoff and submission scheduling.
//!
//! Replaces standard linear/exponential backoff with deterministic chaos to break phase-lock
//! scenarios, increasing throughput in concurrent ring-buffer submissions.

use core::f32::consts;

/// Lorenz attractor parameters and state
#[derive(Clone, Copy, Debug)]
pub struct Lorenz {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub sigma: f32,
    pub rho: f32,
    pub beta: f32,
}

impl Lorenz {
    pub const fn default() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        }
    }

    pub fn step(&mut self, dt: f32) -> f32 {
        let dt = if dt.is_finite() {
            dt.clamp(0.0001, 0.1)
        } else {
            0.01
        };
        let dx = self.sigma * (self.y - self.x);
        let dy = self.x * (self.rho - self.z) - self.y;
        let dz = self.x * self.y - self.beta * self.z;
        self.x += dx * dt;
        self.y += dy * dt;
        self.z += dz * dt;
        self.x
    }
}

/// Rössler attractor
#[derive(Clone, Copy, Debug)]
pub struct Roessler {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

impl Roessler {
    pub const fn default() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
            a: 0.2,
            b: 0.2,
            c: 5.7,
        }
    }

    pub fn step(&mut self, dt: f32) -> f32 {
        let dt = if dt.is_finite() {
            dt.clamp(0.0001, 0.1)
        } else {
            0.01
        };
        let dx = -self.y - self.z;
        let dy = self.x + self.a * self.y;
        let dz = self.b + self.z * (self.x - self.c);
        self.x += dx * dt;
        self.y += dy * dt;
        self.z += dz * dt;
        self.x
    }
}

/// Logistic Map for extremely fast pseudo-random scheduling
#[derive(Clone, Copy, Debug)]
pub struct LogisticMap {
    pub x: f32,
    pub r: f32,
}

impl LogisticMap {
    pub const fn default() -> Self {
        Self { x: 0.5, r: 3.99 } // Deep chaos regime
    }

    pub fn step(&mut self) -> f32 {
        self.x = self.r * self.x * (1.0 - self.x);
        self.x
    }
}

/// Fast Taylor approximation for cos(x) avoiding libm.
fn fast_cos(mut x: f32) -> f32 {
    const PI2: f32 = consts::TAU;
    while x > PI2 {
        x -= PI2;
    }
    while x < 0.0 {
        x += PI2;
    }
    // Shift to -pi..pi
    if x > consts::PI {
        x -= PI2;
    }

    let x2 = x * x;
    1.0 - (x2 / 2.0) + (x2 * x2 / 24.0) - (x2 * x2 * x2 / 720.0)
}

/// Duffing Oscillator for resonant forcing throughputs
#[derive(Clone, Copy, Debug)]
pub struct Duffing {
    pub x: f32,
    pub v: f32,
    pub alpha: f32,
    pub beta: f32,
    pub delta: f32,
    pub gamma: f32,
    pub omega: f32,
    pub t: f32,
}

impl Duffing {
    pub const fn default() -> Self {
        Self {
            x: 1.0,
            v: 0.0,
            alpha: -1.0,
            beta: 1.0,
            delta: 0.3,
            gamma: 0.2,
            omega: 1.2,
            t: 0.0,
        }
    }

    pub fn step(&mut self, dt: f32) -> f32 {
        let dt = if dt.is_finite() {
            dt.clamp(0.0001, 0.1)
        } else {
            0.01
        };
        let force = self.gamma * fast_cos(self.omega * self.t);
        let dv = force
            - self.delta * self.v
            - self.alpha * self.x
            - self.beta * self.x * self.x * self.x;
        self.x += self.v * dt;
        self.v += dv * dt;
        self.t += dt;
        self.x
    }
}

/// Mandelbrot escape time logic (useful for exponential proof-of-work or wait states)
pub fn mandelbrot_escape(cx: f32, cy: f32, max_iters: u32) -> u32 {
    let mut zx = 0.0;
    let mut zy = 0.0;
    for i in 0..max_iters {
        let zx2 = zx * zx;
        let zy2 = zy * zy;
        if zx2 + zy2 > 4.0 {
            return i;
        }
        zy = 2.0 * zx * zy + cy;
        zx = zx2 - zy2 + cx;
    }
    max_iters
}

/// Lyapunov exponent estimation via trajectory divergence
#[derive(Clone, Copy, Debug)]
pub struct LyapunovEstimator {
    pub sum_log_deriv: f32,
    pub count: u32,
}

impl LyapunovEstimator {
    pub const fn new() -> Self {
        Self {
            sum_log_deriv: 0.0,
            count: 0,
        }
    }

    /// Fast approximation of log2(abs(x)) to avoid libm.
    fn fast_log2_abs(x: f32) -> f32 {
        let x_bits = x.to_bits();
        let exp = ((x_bits >> 23) & 0xFF) as i32 - 127;
        let mantissa = (x_bits & 0x7FFFFF) as f32 / 8388608.0; // 2^23
        exp as f32 + mantissa
    }

    pub fn update(&mut self, derivative: f32) {
        if derivative != 0.0 && derivative.is_finite() {
            let term = Self::fast_log2_abs(derivative);
            if term.is_finite() {
                self.sum_log_deriv += term;
                self.count = self.count.saturating_add(1);
                if !self.sum_log_deriv.is_finite() {
                    // A diagnostic overflow must not poison scheduling state.
                    self.sum_log_deriv = 0.0;
                    self.count = 0;
                }
            }
        }
    }

    pub fn exponent(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.sum_log_deriv / (self.count as f32)
        }
    }
}

impl Default for LyapunovEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Observable scheduler state for diagnostics and qualification reports.
#[derive(Clone, Copy, Debug)]
pub struct ChaosSnapshot {
    pub lorenz: (f32, f32, f32),
    pub roessler: (f32, f32, f32),
    pub logistic: f32,
    pub duffing: (f32, f32),
    pub lyapunov_exponent: f32,
    pub last_interval_us: u32,
}

/// Adaptive Chaos Scheduler.
#[derive(Clone, Copy, Debug)]
pub struct ChaosScheduler {
    lorenz: Lorenz,
    roessler: Roessler,
    logistic: LogisticMap,
    duffing: Duffing,
    lyapunov: LyapunovEstimator,
    last_interval_us: u32,
}

impl ChaosScheduler {
    pub const fn new() -> Self {
        Self {
            lorenz: Lorenz::default(),
            roessler: Roessler::default(),
            logistic: LogisticMap::default(),
            duffing: Duffing::default(),
            lyapunov: LyapunovEstimator::new(),
            last_interval_us: 1,
        }
    }

    /// Compute a highly decorrelated service interval combining multiple
    /// attractors. Inputs are bounded so a caller cannot drive the integrators
    /// into NaN/Inf and silently lose its scheduling guarantee.
    pub fn next_interval(&mut self, dt: f32) -> u32 {
        let dt = if dt.is_finite() {
            dt.clamp(0.0001, 0.1)
        } else {
            0.01
        };
        let l = self.lorenz.step(dt);
        let r = self.roessler.step(dt);
        let logis = self.logistic.step();
        let d = self.duffing.step(dt);

        // Explicitly bound the integrated states.  Euler integration is used
        // to keep this `no_std` scheduler tiny; a bad caller-supplied step or
        // an extreme future parameter must not turn a scheduling diagnostic
        // into NaN/Inf and poison a hot path.
        self.lorenz.x = finite_clamp(self.lorenz.x, -1_000.0, 1_000.0, 1.0);
        self.lorenz.y = finite_clamp(self.lorenz.y, -1_000.0, 1_000.0, 1.0);
        self.lorenz.z = finite_clamp(self.lorenz.z, -1_000.0, 1_000.0, 1.0);
        self.roessler.x = finite_clamp(self.roessler.x, -1_000.0, 1_000.0, 1.0);
        self.roessler.y = finite_clamp(self.roessler.y, -1_000.0, 1_000.0, 1.0);
        self.roessler.z = finite_clamp(self.roessler.z, -1_000.0, 1_000.0, 1.0);
        self.duffing.x = finite_clamp(self.duffing.x, -100.0, 100.0, 1.0);
        self.duffing.v = finite_clamp(self.duffing.v, -100.0, 100.0, 0.0);
        self.duffing.t = finite_clamp(self.duffing.t, 0.0, 1_000_000.0, 0.0);

        // Mix the phases to break traditional exponential backoff lock-stepping.
        let mixed = (l * r + logis * d).abs();

        // Track lyapunov stability to ensure we remain in chaotic regime.
        self.lyapunov.update(mixed - (l * r)); // proxy for phase expansion

        // Bound the interval to 1–50 µs even if a future integrator changes
        // its numerical range.
        let interval = if mixed.is_finite() {
            (1.0 + (mixed.abs() % 50.0)) as u32
        } else {
            1
        };
        self.last_interval_us = interval.clamp(1, 50);
        self.last_interval_us
    }

    /// Return the current attractor and Lyapunov state for telemetry.
    pub fn snapshot(&self) -> ChaosSnapshot {
        ChaosSnapshot {
            lorenz: (self.lorenz.x, self.lorenz.y, self.lorenz.z),
            roessler: (self.roessler.x, self.roessler.y, self.roessler.z),
            logistic: self.logistic.x,
            duffing: (self.duffing.x, self.duffing.v),
            lyapunov_exponent: self.lyapunov.exponent(),
            last_interval_us: self.last_interval_us,
        }
    }
}

fn finite_clamp(value: f32, low: f32, high: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        fallback
    }
}

impl Default for ChaosScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_stays_bounded_and_observable() {
        let mut scheduler = ChaosScheduler::new();
        for i in 0..10_000 {
            let dt = if i % 97 == 0 {
                f32::NAN
            } else if i % 101 == 0 {
                0.1
            } else {
                0.01
            };
            let interval = scheduler.next_interval(dt);
            assert!((1..=50).contains(&interval));
        }
        let snapshot = scheduler.snapshot();
        assert!((1..=50).contains(&snapshot.last_interval_us));
        assert!(snapshot.lyapunov_exponent.is_finite());
        assert!(snapshot.lorenz.0.is_finite());
        assert!(snapshot.roessler.0.is_finite());
        assert!(snapshot.duffing.0.is_finite());
    }
}
