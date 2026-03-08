---
triggers: ["Julia", "julia scientific", "DifferentialEquations.jl", "Flux.jl", "Plots.jl", "DataFrames.jl", "julia package", "julia performance", "julia type system"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["julia"]
category: julia
---

# Julia Scientific Computing

When writing Julia code for scientific computing and numerical analysis:

1. Use Julia's type system for performance: annotate function arguments with abstract types `function solve(f::Function, x0::AbstractVector{<:Real})` — the compiler specializes for concrete types at call time.
2. Avoid global variables in performance-critical code — wrap computations in functions; if globals are needed, declare them `const` or use type annotations: `global x::Float64 = 0.0`.
3. Use `DifferentialEquations.jl` for ODE/PDE solving: `prob = ODEProblem(f!, u0, tspan, p); sol = solve(prob, Tsit5())` — choose solvers by stiffness: `Tsit5()` for non-stiff, `Rodas5P()` for stiff systems.
4. Use `DataFrames.jl` for tabular data: `df = DataFrame(CSV.File("data.csv"))` — chain operations with `@chain` from `DataFramesMeta.jl`: `@chain df begin @subset(:age .> 30) @select(:name, :salary) end`.
5. Plot with `Plots.jl` using backends: `using Plots; gr()` for fast interactive, `pgfplotsx()` for LaTeX-quality publication figures — `plot(x, y, label="data", xlabel="t", ylabel="x(t)", lw=2)`.
6. Use `Flux.jl` for machine learning: `model = Chain(Dense(784, 128, relu), Dense(128, 10), softmax); loss(x, y) = crossentropy(model(x), y)` — train with `Flux.train!(loss, params(model), data, opt)`.
7. Leverage multiple dispatch: define methods for different input types `simulate(model::LinearModel, data)` and `simulate(model::NonlinearModel, data)` — Julia dispatches to the correct method at runtime.
8. Use `@benchmark` from `BenchmarkTools.jl` for accurate timing: `@benchmark solve($prob)` — the `$` interpolation prevents benchmarking global variable access instead of the actual computation.
9. Write type-stable functions: avoid containers that change element type — use `Vector{Float64}` not `Vector{Any}`; check with `@code_warntype myfunction(args...)` for red-highlighted `Any` types.
10. Use `LinearAlgebra` stdlib for matrix operations: `eigen(A)`, `svd(A)`, `lu(A)`, `qr(A)` — Julia matrices are column-major (Fortran order), so iterate columns in the inner loop for cache efficiency.
11. Parallelize with `Threads.@threads for i in 1:N` for shared-memory or `Distributed` + `pmap(f, collection)` for multi-process — use `@sync @async` for concurrent I/O tasks.
12. Create packages with `Pkg.generate("MyPackage")` — structure as `src/MyPackage.jl` with `module MyPackage; export func1, func2; include("utils.jl"); end` and tests in `test/runtests.jl`.
13. Use `Makie.jl` (CairoMakie, GLMakie) for advanced visualization: `fig = Figure(); ax = Axis(fig[1,1]); lines!(ax, x, y)` — supports 3D plots, animations, and interactive exploration with GLMakie.
14. Handle units with `Unitful.jl`: `velocity = 3.0u"m/s"; time = 2.0u"s"; distance = velocity * time` — unit errors are caught at compile time, preventing physics/engineering bugs.
