---
triggers: ["scipy", "scientific python", "numerical computing", "simulation", "signal processing", "optimization python", "sympy", "symbolic math", "ODE solver python", "FFT python"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Python Scientific Computing

When using Python for scientific computing, simulations, and numerical analysis:

1. Use `scipy.integrate.solve_ivp()` for ODE solving: `sol = solve_ivp(rhs, t_span, y0, method='RK45', dense_output=True)` — choose `RK45` for non-stiff, `Radau` or `BDF` for stiff systems.
2. Use `scipy.optimize.minimize()` for optimization: `result = minimize(objective, x0, method='L-BFGS-B', bounds=bounds)` — specify gradients with `jac` for faster convergence; use `differential_evolution()` for global optimization.
3. Perform FFT analysis with `scipy.fft`: `freqs = fftfreq(N, d=1/sample_rate); spectrum = fft(signal)` — use `rfft()` for real-valued signals (2x faster, half the output) and `fftshift()` for centered spectra.
4. Use `scipy.signal` for signal processing: `sos = signal.butter(4, [0.1, 0.4], btype='band', output='sos'); filtered = signal.sosfilt(sos, data)` — always use SOS (second-order sections) format over transfer functions for numerical stability.
5. Solve linear systems with `scipy.linalg.solve(A, b)` instead of `np.linalg.inv(A) @ b` — direct solve is faster and numerically more stable; use `scipy.sparse.linalg.spsolve()` for sparse systems.
6. Use `SymPy` for symbolic math: `from sympy import symbols, diff, integrate, solve; x = symbols('x'); solve(x**2 - 4, x)` — derive analytical solutions, then convert to numerical functions with `lambdify(x, expr, 'numpy')`.
7. Interpolate data with `scipy.interpolate`: `f = interp1d(x, y, kind='cubic')` for 1D; `RegularGridInterpolator((x, y), Z)` for N-D — avoid extrapolation; set `fill_value='extrapolate'` only when justified.
8. Use `numpy` broadcasting for vectorized computation: `distances = np.sqrt(np.sum((points[:, np.newaxis] - centroids[np.newaxis, :]) ** 2, axis=-1))` — eliminates explicit loops over data points.
9. Run Monte Carlo simulations with `np.random.Generator`: `rng = np.random.default_rng(42); samples = rng.normal(mu, sigma, size=(N, dim))` — use the new Generator API (not legacy `np.random.randn`).
10. Use `scipy.stats` for distributions: `stats.norm.pdf(x, loc=0, scale=1)`, `stats.norm.fit(data)` for parameter estimation, `stats.ks_2samp(a, b)` for distribution comparison — 100+ distributions available.
11. Compute sparse matrices with `scipy.sparse`: `A = sparse.csr_matrix((data, (row, col)), shape=(m, n))` — use CSR for row slicing/matrix-vector products, CSC for column operations, COO for construction.
12. Use `numba` for JIT-compiled numerical functions: `@numba.jit(nopython=True) def compute(x): ...` — gives C-like speed for loops over numpy arrays; use `@numba.vectorize` for custom ufuncs.
13. Parallelize with `joblib`: `Parallel(n_jobs=-1)(delayed(func)(i) for i in range(N))` — simpler than `multiprocessing` for embarrassingly parallel tasks; integrates with scikit-learn.
14. Use `h5py` for large numerical datasets: `with h5py.File('data.h5', 'w') as f: f.create_dataset('results', data=array, compression='gzip')` — HDF5 supports partial reads, chunking, and compression for TB-scale data.
