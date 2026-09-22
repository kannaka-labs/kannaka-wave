//! The latent predictor E-004 fixes: a 2-layer MLP over the concatenated
//! context of `k` event embeddings, predicting the next embedding, trained
//! with mean squared error in the encoder's space. Adam, minibatches, a
//! seeded initialisation. No dependencies; the sizes are small enough that
//! scalar loops on one CPU are minutes, not hours.
//!
//! The seed matters more here than the spec expected: the substrate's dream is
//! deterministic (`VectorStore::dream_at` has no randomness), so "a seed seeds
//! only the dream" seeds nothing. Here it seeds this predictor's weights and
//! minibatch order, which is the only stochastic part of arm S, and arm U is
//! identical across seeds. The report says so.

/// xorshift64*, seeded. Enough for weights and shuffles; not for anything
/// that has to be unpredictable.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Standard normal, Box–Muller.
    pub fn normal(&mut self) -> f32 {
        let u1 = self.unit().max(1e-7);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// A dense layer with its Adam state.
struct Layer {
    inp: usize,
    out: usize,
    w: Vec<f32>, // out × inp, row-major
    b: Vec<f32>,
    mw: Vec<f32>,
    vw: Vec<f32>,
    mb: Vec<f32>,
    vb: Vec<f32>,
}

impl Layer {
    fn new(inp: usize, out: usize, rng: &mut Rng) -> Layer {
        // He-uniform-ish scale for the hidden layer; the output layer gets
        // the same and relies on Adam.
        let scale = (2.0 / inp as f32).sqrt();
        let w = (0..inp * out).map(|_| rng.normal() * scale).collect();
        Layer {
            inp,
            out,
            w,
            b: vec![0.0; out],
            mw: vec![0.0; inp * out],
            vw: vec![0.0; inp * out],
            mb: vec![0.0; out],
            vb: vec![0.0; out],
        }
    }

    fn forward(&self, x: &[f32], y: &mut [f32]) {
        for (o, yo) in y.iter_mut().enumerate().take(self.out) {
            let row = &self.w[o * self.inp..(o + 1) * self.inp];
            let mut acc = self.b[o];
            for (wi, xi) in row.iter().zip(x) {
                acc += wi * xi;
            }
            *yo = acc;
        }
    }
}

/// Gradient accumulators for one minibatch.
struct Grads {
    w1: Vec<f32>,
    b1: Vec<f32>,
    w2: Vec<f32>,
    b2: Vec<f32>,
}

/// The predictor. `k` context vectors of `dims` each, concatenated.
pub struct Mlp {
    pub k: usize,
    pub dims: usize,
    pub hidden: usize,
    l1: Layer,
    l2: Layer,
    step: u64,
}

impl Mlp {
    pub fn new(k: usize, dims: usize, hidden: usize, seed: u64) -> Mlp {
        let mut rng = Rng::new(seed);
        Mlp {
            k,
            dims,
            hidden,
            l1: Layer::new(k * dims, hidden, &mut rng),
            l2: Layer::new(hidden, dims, &mut rng),
            step: 0,
        }
    }

    /// Ẑ for a context of exactly `k` vectors, oldest first.
    pub fn predict(&self, context: &[&[f32]]) -> Vec<f32> {
        let x = self.concat(context);
        let mut h = vec![0.0; self.hidden];
        self.l1.forward(&x, &mut h);
        for v in h.iter_mut() {
            *v = v.max(0.0);
        }
        let mut y = vec![0.0; self.dims];
        self.l2.forward(&h, &mut y);
        y
    }

    fn concat(&self, context: &[&[f32]]) -> Vec<f32> {
        assert_eq!(context.len(), self.k, "context length");
        let mut x = Vec::with_capacity(self.k * self.dims);
        for c in context {
            assert_eq!(c.len(), self.dims, "vector dims");
            x.extend_from_slice(c);
        }
        x
    }

    /// One epoch of Adam over `(context, target)` samples in a seeded order.
    /// Returns the mean MSE over the epoch (before each update).
    pub fn train_epoch(
        &mut self,
        samples: &[(Vec<&[f32]>, &[f32])],
        batch: usize,
        lr: f32,
        rng: &mut Rng,
    ) -> f32 {
        let mut order: Vec<usize> = (0..samples.len()).collect();
        rng.shuffle(&mut order);
        let mut total = 0.0;
        for chunk in order.chunks(batch) {
            let mut g = Grads {
                w1: vec![0.0; self.l1.w.len()],
                b1: vec![0.0; self.hidden],
                w2: vec![0.0; self.l2.w.len()],
                b2: vec![0.0; self.dims],
            };
            for &i in chunk {
                let (ctx, target) = &samples[i];
                total += self.backward(ctx, target, &mut g) / samples.len() as f32;
            }
            let n = chunk.len() as f32;
            self.step += 1;
            adam(
                &mut self.l1.w,
                &mut self.l1.mw,
                &mut self.l1.vw,
                &g.w1,
                n,
                lr,
                self.step,
            );
            adam(
                &mut self.l1.b,
                &mut self.l1.mb,
                &mut self.l1.vb,
                &g.b1,
                n,
                lr,
                self.step,
            );
            adam(
                &mut self.l2.w,
                &mut self.l2.mw,
                &mut self.l2.vw,
                &g.w2,
                n,
                lr,
                self.step,
            );
            adam(
                &mut self.l2.b,
                &mut self.l2.mb,
                &mut self.l2.vb,
                &g.b2,
                n,
                lr,
                self.step,
            );
        }
        total
    }

    /// Forward, then accumulate gradients of ½·MSE for one sample. Returns
    /// the sample's MSE.
    fn backward(&self, ctx: &[&[f32]], target: &[f32], g: &mut Grads) -> f32 {
        let x = self.concat(ctx);
        let mut pre = vec![0.0; self.hidden];
        self.l1.forward(&x, &mut pre);
        let h: Vec<f32> = pre.iter().map(|v| v.max(0.0)).collect();
        let mut y = vec![0.0; self.dims];
        self.l2.forward(&h, &mut y);
        // dL/dy for L = mean((y − t)²) over dims.
        let scale = 2.0 / self.dims as f32;
        let dy: Vec<f32> = y.iter().zip(target).map(|(a, b)| scale * (a - b)).collect();
        let mse = y
            .iter()
            .zip(target)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            / self.dims as f32;
        // Layer 2 grads and dh.
        let mut dh = vec![0.0; self.hidden];
        for (o, &d) in dy.iter().enumerate() {
            g.b2[o] += d;
            let row = &mut g.w2[o * self.hidden..(o + 1) * self.hidden];
            let wrow = &self.l2.w[o * self.hidden..(o + 1) * self.hidden];
            for j in 0..self.hidden {
                row[j] += d * h[j];
                dh[j] += d * wrow[j];
            }
        }
        // ReLU and layer 1.
        for j in 0..self.hidden {
            if pre[j] <= 0.0 {
                continue;
            }
            let d = dh[j];
            g.b1[j] += d;
            let row = &mut g.w1[j * self.l1.inp..(j + 1) * self.l1.inp];
            for (r, xi) in row.iter_mut().zip(&x) {
                *r += d * xi;
            }
        }
        mse
    }
}

fn adam(p: &mut [f32], m: &mut [f32], v: &mut [f32], g: &[f32], n: f32, lr: f32, t: u64) {
    const B1: f32 = 0.9;
    const B2: f32 = 0.999;
    const EPS: f32 = 1e-8;
    let c1 = 1.0 - B1.powi(t as i32);
    let c2 = 1.0 - B2.powi(t as i32);
    for i in 0..p.len() {
        let gi = g[i] / n;
        m[i] = B1 * m[i] + (1.0 - B1) * gi;
        v[i] = B2 * v[i] + (1.0 - B2) * gi * gi;
        p[i] -= lr * (m[i] / c1) / ((v[i] / c2).sqrt() + EPS);
    }
}

/// Mean squared error between two vectors, in the encoder's space.
pub fn mse(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f32>() / a.len() as f32
}

/// Euclidean distance: the raw surprise drive `d(Ẑ, Z)`.
pub fn dist(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_weights_and_the_gradient_reduces_loss() {
        let ctx: Vec<Vec<f32>> = (0..3)
            .map(|i| vec![0.1 * i as f32, 0.2, -0.3, 0.4])
            .collect();
        let target = vec![0.5, -0.5, 0.25, 0.0];
        let refs: Vec<&[f32]> = ctx.iter().map(|v| v.as_slice()).collect();
        let a = Mlp::new(3, 4, 8, 7);
        let b = Mlp::new(3, 4, 8, 7);
        assert_eq!(a.predict(&refs), b.predict(&refs));
        let mut m = Mlp::new(3, 4, 8, 7);
        let mut rng = Rng::new(1);
        let samples = vec![(refs.clone(), target.as_slice())];
        let before = mse(&m.predict(&refs), &target);
        for _ in 0..200 {
            m.train_epoch(&samples, 1, 1e-2, &mut rng);
        }
        let after = mse(&m.predict(&refs), &target);
        assert!(after < before * 0.1, "{before} -> {after}");
    }
}
