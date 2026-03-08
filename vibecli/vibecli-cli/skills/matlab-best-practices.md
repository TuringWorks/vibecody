---
triggers: ["MATLAB", "Simulink", "matlab script", "matlab function", ".m file", "MEX", "matlab toolbox", "matlab plot", "matlab matrix"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["matlab"]
category: matlab
---

# MATLAB Best Practices

When writing MATLAB code for engineering and scientific computing:

1. Vectorize operations instead of loops: `result = A .* B + C` is orders of magnitude faster than element-wise for-loops — MATLAB's strength is matrix/vector operations via BLAS/LAPACK.
2. Pre-allocate arrays before loops: `result = zeros(1, N)` — growing arrays in a loop with `result = [result, newVal]` causes O(N^2) memory copies and dramatic slowdowns.
3. Use functions (`.m` files) over scripts for reusable code: `function [out1, out2] = myFunc(in1, in2)` — functions have their own workspace scope, scripts pollute the base workspace.
4. Use `logical indexing` for filtering: `data(data > threshold)` instead of `find()` + indexing — it's both faster and more readable for conditional selection.
5. Handle matrix dimensions carefully: use `size(A, 1)` for rows, `size(A, 2)` for columns; `length()` returns the largest dimension which can cause subtle bugs — prefer `numel()` for element count.
6. Use `containers.Map` or `struct` for key-value data instead of parallel arrays — `params = struct('lr', 0.01, 'epochs', 100, 'batch_size', 32)` keeps related parameters together.
7. Plot with best practices: `figure; hold on; plot(x, y1, 'b-', 'LineWidth', 1.5); xlabel('Time (s)'); ylabel('Amplitude'); title('Signal'); legend('Channel 1'); grid on` — always label axes with units.
8. Use `try-catch` for error handling: `try ... catch ME, fprintf('Error: %s\n', ME.message); end` — especially around file I/O and external function calls.
9. Profile performance with `profile on; myFunction(); profile viewer` — identifies bottleneck functions; use `tic/toc` for quick timing of specific sections.
10. Use `parfor` for embarrassingly parallel loops: `parfor i = 1:N, result(i) = expensiveFunc(data(i)); end` — requires Parallel Computing Toolbox; iterations must be independent.
11. Save/load data with `.mat` files: `save('results.mat', 'data', 'params', '-v7.3')` for HDF5-based files that support >2 GB variables; use `-v7` for compatibility with older MATLAB versions.
12. Use `cellfun` and `arrayfun` for applying functions to cell arrays: `cellfun(@(x) x.^2, myCell, 'UniformOutput', false)` — set `UniformOutput` to `false` when outputs have different sizes.
13. Write unit tests with MATLAB's testing framework: `classdef TestMyFunc < matlab.unittest.TestCase; methods(Test); function testBasic(tc), tc.verifyEqual(myFunc(2), 4); end; end; end`.
14. Use `addpath(genpath('src'))` at project startup to add all subdirectories — create a `startup.m` in the project root for consistent path configuration across team members.
15. Interface with Python using `py.module.function()`: `py.numpy.array([1,2,3])` — useful for leveraging Python ML libraries from MATLAB; use `pyenv('Version', '/path/to/python')` to set the interpreter.
